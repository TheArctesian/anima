//! The sealed form of HTTP secrets: bound to their check by the vault context, and to the
//! destination they were given for by the plaintext (SEC-003, SEC-005).
//!
//! Plaintext layout: a 4-byte big-endian length, that many bytes of binding JSON (the URL
//! origin, the secret header names, the authentication and whether TLS was verified), then
//! the encoded secrets. The binding is checked against the stored configuration on every
//! open, with the rule an update uses to keep secrets, so a database writer who points the
//! URL elsewhere, renames a secret header, changes the authentication or turns TLS checks off
//! gets an error instead of the secrets.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    checks::{
        Error,
        config::{HttpAuth, HttpConfig, HttpSecrets},
        rules::origin,
    },
    credentials::{self, Sealed, Secret, Vault},
    ids::CheckId,
};

/// The associated data that binds sealed HTTP secrets to their check (SEC-003).
fn context(check: CheckId) -> Vec<u8> {
    format!("checks.checks.http:{check}").into_bytes()
}

/// Where the secrets may be sent: the URL origin, the secret header names (lower case, in
/// order), the authentication kind and user name, and whether TLS was verified.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    origin: String,
    headers: Vec<String>,
    auth: HttpAuth,
    verify_tls: bool,
}

impl Binding {
    fn of(config: &HttpConfig) -> Self {
        Self {
            origin: origin(&config.url).unwrap_or_default(),
            headers: config
                .headers
                .iter()
                .filter(|header| header.value.is_none())
                .map(|header| header.name.to_ascii_lowercase())
                .collect(),
            auth: config.auth.clone(),
            verify_tls: config.verify_tls,
        }
    }

    /// Whether the secrets may go where `config` sends them: everything the same, except
    /// that TLS checks may be turned on (SEC-005).
    fn permits(&self, config: &HttpConfig) -> bool {
        let now = Self::of(config);
        !self.origin.is_empty()
            && self.origin == now.origin
            && self.headers == now.headers
            && self.auth == now.auth
            && (now.verify_tls || !self.verify_tls)
    }
}

pub(super) fn malformed() -> Error {
    Error::Credentials(credentials::Error::Malformed)
}

/// The plaintext to seal: a 4-byte big-endian length, the binding, then the secrets.
pub(super) fn frame(binding: &[u8], secrets: &Secret) -> Secret {
    let length = u32::try_from(binding.len()).unwrap_or(u32::MAX);
    let mut plain = Zeroizing::new(Vec::with_capacity(
        4 + binding.len() + secrets.expose().len(),
    ));
    plain.extend_from_slice(&length.to_be_bytes());
    plain.extend_from_slice(binding);
    plain.extend_from_slice(secrets.expose());
    Secret::new(std::mem::take(&mut *plain))
}

/// The binding and the secrets of an opened [`frame`].
///
/// # Errors
/// `Credentials` (`Malformed`) when the length is past the plaintext.
pub(super) fn unframe(plain: &[u8]) -> Result<(&[u8], &[u8]), Error> {
    let (length, rest) = plain.split_first_chunk::<4>().ok_or_else(malformed)?;
    let length = usize::try_from(u32::from_be_bytes(*length)).map_err(|_| malformed())?;
    if length > rest.len() {
        return Err(malformed());
    }
    Ok(rest.split_at(length))
}

/// Seals the secrets of `config` for the check.
pub(in crate::checks) fn seal(
    vault: &Vault,
    check: CheckId,
    config: &HttpConfig,
    secrets: &HttpSecrets,
) -> Vec<u8> {
    let binding = serde_json::to_vec(&Binding::of(config)).unwrap_or_default();
    let plain = frame(&binding, &secrets.encode());
    vault.seal(&plain, &context(check)).to_bytes()
}

/// Opens the check's secrets, if they were sealed for the destination `config` names.
///
/// # Errors
/// `Credentials` when the value does not open or is malformed; `SecretsUnbound` when the
/// configuration no longer names the origin and secret headers they were given for.
pub(in crate::checks) fn open(
    vault: &Vault,
    check: CheckId,
    config: &HttpConfig,
    bytes: &[u8],
) -> Result<HttpSecrets, Error> {
    let plain = vault.open(&Sealed::from_bytes(bytes)?, &context(check))?;
    let (binding, secrets) = unframe(plain.expose())?;
    let binding: Binding = serde_json::from_slice(binding).map_err(|_| malformed())?;
    if !binding.permits(config) {
        return Err(Error::SecretsUnbound);
    }
    HttpSecrets::decode(&Secret::new(secrets.to_vec())).map_err(|_| malformed())
}
