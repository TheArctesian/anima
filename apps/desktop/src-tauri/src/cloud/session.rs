use keyring::Entry;
use zeroize::Zeroizing;

use super::CloudError;
use crate::keystore::SERVICE;

/// The `WorkOS` refresh token of this app, one for each client ID, in the OS key store (HMD-034).
/// It never reaches the web view, a file or a log (LOG-007).
fn entry(client_id: &str) -> Result<Entry, CloudError> {
    Entry::new(&format!("{SERVICE}.cloud"), client_id).map_err(|_| CloudError::Keystore)
}

pub(super) fn read(client_id: &str) -> Result<Option<Zeroizing<String>>, CloudError> {
    match entry(client_id)?.get_password() {
        Ok(token) => Ok(Some(Zeroizing::new(token))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(CloudError::Keystore),
    }
}

/// Keeps `token` in place of the stored one: `WorkOS` may rotate it at each exchange.
pub(super) fn write(client_id: &str, token: &str) -> Result<(), CloudError> {
    entry(client_id)?
        .set_password(token)
        .map_err(|_| CloudError::Keystore)
}

/// Deletes the stored token, as when `WorkOS` refuses it (HMD-034).
pub(super) fn clear(client_id: &str) -> Result<(), CloudError> {
    match entry(client_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(CloudError::Keystore),
    }
}
