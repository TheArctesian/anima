//! The sealed form of SNMP secrets: bound to their check by the vault context, and to the
//! version and user name they were given for by the plaintext (SEC-003, SEC-005, D-5).
//!
//! Plaintext layout: the frame of `sealed` with binding JSON (`version`, `username`), then
//! the secrets: one kind byte (1 community, 2 v3 user), and each secret as a 2-byte
//! big-endian length and its bytes. The binding is checked against the stored
//! configuration on every open, so a database writer who changes the version or the user
//! name gets an error instead of the secrets, and nothing is sent.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::sealed::{frame, malformed, unframe};
use crate::{
    checks::{
        Error,
        config::{SnmpConfig, SnmpSecrets, SnmpVersion},
    },
    credentials::{Sealed, Secret, Vault},
    ids::CheckId,
};

const COMMUNITY: u8 = 1;
const USER: u8 = 2;

/// The associated data that binds sealed SNMP secrets to their check (SEC-003).
fn context(check: CheckId) -> Vec<u8> {
    format!("checks.checks.snmp:{check}").into_bytes()
}

/// What the secrets were given for.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    version: SnmpVersion,
    username: Option<String>,
}

impl Binding {
    fn of(config: &SnmpConfig) -> Self {
        Self {
            version: config.version,
            username: config.username.clone(),
        }
    }
}

/// The secrets in the sealed layout, in a buffer sized up front so it never grows.
fn encode(secrets: &SnmpSecrets) -> Secret {
    let (kind, parts): (u8, Vec<&Secret>) = match secrets {
        SnmpSecrets::Community(community) => (COMMUNITY, vec![community]),
        SnmpSecrets::User { auth, privacy } => (USER, vec![auth, privacy]),
    };
    let size = 1 + parts.iter().map(|p| 2 + p.expose().len()).sum::<usize>();
    let mut bytes = Zeroizing::new(Vec::with_capacity(size));
    bytes.push(kind);
    for part in parts {
        let length = u16::try_from(part.expose().len()).unwrap_or(u16::MAX);
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(&part.expose()[..usize::from(length)]);
    }
    Secret::new(std::mem::take(&mut *bytes))
}

/// The next length-prefixed secret.
fn part(bytes: &mut &[u8]) -> Result<Secret, Error> {
    let (length, rest) = bytes.split_first_chunk::<2>().ok_or_else(malformed)?;
    let length = usize::from(u16::from_be_bytes(*length));
    if length > rest.len() {
        return Err(malformed());
    }
    let (value, rest) = rest.split_at(length);
    *bytes = rest;
    Ok(Secret::new(value.to_vec()))
}

/// Reads back what [`encode`] wrote; every byte must be used.
fn decode(bytes: &[u8]) -> Result<SnmpSecrets, Error> {
    let (&kind, mut rest) = bytes.split_first().ok_or_else(malformed)?;
    let secrets = match kind {
        COMMUNITY => SnmpSecrets::Community(part(&mut rest)?),
        USER => SnmpSecrets::User {
            auth: part(&mut rest)?,
            privacy: part(&mut rest)?,
        },
        _ => return Err(malformed()),
    };
    if rest.is_empty() {
        Ok(secrets)
    } else {
        Err(malformed())
    }
}

/// Seals the secrets of `config` for the check.
///
/// # Errors
/// `Credentials` (`Random`) when the OS random source fails.
pub(in crate::checks) fn seal(
    vault: &Vault,
    check: CheckId,
    config: &SnmpConfig,
    secrets: &SnmpSecrets,
) -> Result<Vec<u8>, Error> {
    let binding = serde_json::to_vec(&Binding::of(config)).unwrap_or_default();
    let plain = frame(&binding, &encode(secrets));
    Ok(vault.seal(&plain, &context(check))?.to_bytes())
}

/// Opens the check's secrets, if they were sealed for the version and user name `config`
/// names.
///
/// # Errors
/// `Credentials` when the value does not open or is malformed; `SecretsUnbound` when the
/// configuration names another version or user name than they were given for.
pub(in crate::checks) fn open(
    vault: &Vault,
    check: CheckId,
    config: &SnmpConfig,
    bytes: &[u8],
) -> Result<SnmpSecrets, Error> {
    let plain = vault.open(&Sealed::from_bytes(bytes)?, &context(check))?;
    let (binding, secrets) = unframe(plain.expose())?;
    let binding: Binding = serde_json::from_slice(binding).map_err(|_| malformed())?;
    if binding != Binding::of(config) {
        return Err(Error::SecretsUnbound);
    }
    let secrets = decode(secrets)?;
    if secrets.version() == config.version {
        Ok(secrets)
    } else {
        Err(malformed())
    }
}
