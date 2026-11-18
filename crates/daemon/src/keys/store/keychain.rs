//! The macOS key store: one generic password in the login Keychain, beside the token's item
//! (K-E5 Section 2.1).

use keyring::Entry;
use zeroize::Zeroizing;

use super::KeyStoreError;
use crate::config::home::Home;

const SERVICE: &str = "ing.anima.daemon";
const ACCOUNT: &str = "key-cache";

pub struct Backend;

impl Backend {
    pub const fn open(_home: &Home) -> Self {
        Self
    }

    #[expect(
        clippy::unused_self,
        reason = "same signature as the file backend on other systems"
    )]
    pub fn read(&self, limit: usize) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
        match entry()?.get_password() {
            Ok(text) => {
                let bytes = Zeroizing::new(text.into_bytes());
                Ok((bytes.len() <= limit).then_some(bytes))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(fault(&error)),
        }
    }

    #[expect(
        clippy::unused_self,
        reason = "same signature as the file backend on other systems"
    )]
    pub fn write(&self, bytes: &[u8]) -> Result<(), KeyStoreError> {
        // The record is JSON, so always UTF-8; anything else is never written.
        let text = std::str::from_utf8(bytes)
            .map_err(|_| KeyStoreError::Keychain("the record is not text".to_owned()))?;
        entry()?.set_password(text).map_err(|error| fault(&error))
    }

    #[expect(
        clippy::unused_self,
        reason = "same signature as the file backend on other systems"
    )]
    pub fn delete(&self) -> Result<(), KeyStoreError> {
        match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(fault(&error)),
        }
    }
}

fn entry() -> Result<Entry, KeyStoreError> {
    Entry::new(SERVICE, ACCOUNT).map_err(|error| fault(&error))
}

/// The Keychain's error as text. An item that is not UTF-8 is named by a fixed sentence: that
/// error carries the item's bytes, which may be a value (LOG-007).
fn fault(error: &keyring::Error) -> KeyStoreError {
    match error {
        keyring::Error::BadEncoding(_) => {
            KeyStoreError::Keychain("the item is not text".to_owned())
        }
        other => KeyStoreError::Keychain(other.to_string()),
    }
}
