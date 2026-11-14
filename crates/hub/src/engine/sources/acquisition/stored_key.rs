//! The plaintext of a sealed App key and its check against the account (K-E1 Section 6).
//!
//! Layout: `{"v":1,"app_id","installation_id","private_key_pem"}`, built in a buffer that is
//! wiped on drop and never grows, as `notifications/sealing.rs` does. `credentials` seals it
//! as opaque bytes, bound to the account id.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::app_key::{AppKey, KeyRefusal};
use crate::fleet::credentials::Secret;

const VERSION: u8 = 1;

#[derive(Serialize)]
struct Stored<'a> {
    v: u8,
    app_id: &'a str,
    installation_id: &'a str,
    private_key_pem: &'a str,
}

#[derive(Deserialize)]
struct Opened {
    v: u8,
    app_id: String,
    installation_id: String,
    private_key_pem: String,
}

/// Why a stored key does not serve the account. Fixed words; the run counts it as `auth`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StoredKeyError {
    #[error("the stored key is not in a known layout")]
    Layout,
    #[error("the stored key is for another installation")]
    OtherInstallation,
    #[error("the stored key is no longer valid: {0}")]
    Refused(KeyRefusal),
}

/// The plaintext to seal for an App key.
///
/// # Panics
/// Never: strings and a number always serialise.
#[must_use]
pub fn to_secret(app_id: &str, installation_id: &str, pem: &str) -> Secret {
    let stored = Stored {
        v: VERSION,
        app_id,
        installation_id,
        private_key_pem: pem,
    };
    // Sized for the worst case (every byte escaped as `\uXXXX`).
    let worst = 6 * (app_id.len() + installation_id.len() + pem.len()) + 128;
    let mut plain = Zeroizing::new(Vec::with_capacity(worst));
    serde_json::to_writer(&mut *plain, &stored)
        .expect("strings and a number always serialise (CQ-023)");
    Secret::new(std::mem::take(&mut *plain))
}

/// Opens the plaintext into a key, and checks that it is the key of `instance_key`, so a
/// database writer cannot point an account at another installation's key.
///
/// # Errors
/// [`StoredKeyError`] for an unknown layout, a key of another installation, or a key that no
/// longer passes the checks.
pub fn from_secret(secret: &Secret, instance_key: &str) -> Result<AppKey, StoredKeyError> {
    let opened: Opened =
        serde_json::from_slice(secret.expose()).map_err(|_| StoredKeyError::Layout)?;
    let pem = Zeroizing::new(opened.private_key_pem);
    if opened.v != VERSION {
        return Err(StoredKeyError::Layout);
    }
    let key = AppKey::read(&opened.app_id, &opened.installation_id, pem.as_bytes())
        .map_err(StoredKeyError::Refused)?;
    if key.instance_key() == instance_key {
        Ok(key)
    } else {
        Err(StoredKeyError::OtherInstallation)
    }
}
