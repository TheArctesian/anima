//! The updater secret key, one per computer, in the OS key store (BLD-045). The text is what
//! `updater_key` makes of the key file; nothing here reads or logs it.

use keyring::Entry;
use zeroize::Zeroizing;

use super::{error::KeystoreError, token::SERVICE};

const ACCOUNT: &str = "updater-key";

fn entry() -> Result<Entry, KeystoreError> {
    Entry::new(SERVICE, ACCOUNT).map_err(KeystoreError::from)
}

/// The stored key text, or `None` when no key is stored.
pub fn read_signing_key() -> Result<Option<Zeroizing<String>>, KeystoreError> {
    match entry()?.get_password() {
        Ok(text) => Ok(Some(Zeroizing::new(text))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Stores the key text, replacing any earlier key. An empty text is refused.
pub fn write_signing_key(text: &str) -> Result<(), KeystoreError> {
    if text.is_empty() {
        return Err(KeystoreError::EmptyToken);
    }
    entry()?.set_password(text).map_err(KeystoreError::from)
}
