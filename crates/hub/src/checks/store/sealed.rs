//! The sealed form of HTTP secrets: bound to their check by the vault context, and to the
//! destination they were given for by the plaintext (SEC-003, SEC-005).
//!
//! Plaintext layout: a 4-byte big-endian length, that many bytes of binding JSON (the URL
//! origin and the secret header names), then the encoded secrets. The binding is checked
//! against the stored configuration on every open, so a database writer who points the URL
//! elsewhere or renames a secret header gets an error instead of the secrets.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    checks::{
        Error,
        config::{HttpConfig, HttpSecrets},
        rules::origin,
    },
    credentials::{self, Sealed, Secret, Vault},
    ids::CheckId,
};

/// The associated data that binds sealed HTTP secrets to their check (SEC-003).
fn context(check: CheckId) -> Vec<u8> {
    format!("checks.checks.http:{check}").into_bytes()
}

/// Where the secrets may be sent: the URL origin and the secret header names, lower case,
/// in order.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    origin: String,
    headers: Vec<String>,
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
        }
    }
}

fn malformed() -> Error {
    Error::Credentials(credentials::Error::Malformed)
}

/// Seals the secrets of `config` for the check.
pub(in crate::checks) fn seal(
    vault: &Vault,
    check: CheckId,
    config: &HttpConfig,
    secrets: &HttpSecrets,
) -> Vec<u8> {
    let binding = serde_json::to_vec(&Binding::of(config)).unwrap_or_default();
    let encoded = secrets.encode();
    let length = u32::try_from(binding.len()).unwrap_or(u32::MAX);
    let mut plain = Zeroizing::new(Vec::with_capacity(
        4 + binding.len() + encoded.expose().len(),
    ));
    plain.extend_from_slice(&length.to_be_bytes());
    plain.extend_from_slice(&binding);
    plain.extend_from_slice(encoded.expose());
    let plain = Secret::new(std::mem::take(&mut *plain));
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
    let bytes = plain.expose();
    let (length, rest) = bytes.split_first_chunk::<4>().ok_or_else(malformed)?;
    let length = usize::try_from(u32::from_be_bytes(*length)).map_err(|_| malformed())?;
    if length > rest.len() {
        return Err(malformed());
    }
    let (binding, secrets) = rest.split_at(length);
    let binding: Binding = serde_json::from_slice(binding).map_err(|_| malformed())?;
    if binding.origin.is_empty() || binding != Binding::of(config) {
        return Err(Error::SecretsUnbound);
    }
    HttpSecrets::decode(&Secret::new(secrets.to_vec())).map_err(|_| malformed())
}
