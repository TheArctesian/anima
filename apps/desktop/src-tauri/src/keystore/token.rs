use keyring::Entry;

use super::error::KeystoreError;

const SERVICE: &str = "ing.anima.desktop";
const SCHEME: &str = "https://";

/// The entry for one hub. The account is the hub origin, so a token is only ever read back for
/// the hub that issued it. The shell does not parse the origin (REPO-003); the client sends the
/// canonical form and this only refuses what cannot be one.
fn entry_for(origin: &str) -> Result<Entry, KeystoreError> {
    if origin.len() <= SCHEME.len() || !origin.starts_with(SCHEME) {
        return Err(KeystoreError::BadOrigin);
    }
    Entry::new(SERVICE, origin).map_err(KeystoreError::from)
}

/// Reads the token stored for `origin`. A missing entry is `None`.
pub fn read_token(origin: &str) -> Result<Option<String>, KeystoreError> {
    match entry_for(origin)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Stores the token for `origin`, replacing any earlier one. An empty token is refused.
pub fn write_token(origin: &str, token: &str) -> Result<(), KeystoreError> {
    let entry = entry_for(origin)?;
    if token.is_empty() {
        return Err(KeystoreError::EmptyToken);
    }
    entry.set_password(token).map_err(KeystoreError::from)
}

/// Removes the token stored for `origin`. A missing entry is already cleared.
pub fn clear_token(origin: &str) -> Result<(), KeystoreError> {
    match entry_for(origin)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
