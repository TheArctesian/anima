//! The macOS token store: one generic password in the login Keychain (MET-013).
//!
//! Compiled only on macOS; it has not been built on this machine (R-092 D-8).

use keyring::Entry;
use zeroize::Zeroizing;

use super::TokenStoreError;
use crate::config::home::Home;

const SERVICE: &str = "ing.anima.agent";
const ACCOUNT: &str = "agent-token";

pub struct Backend;

impl Backend {
    pub const fn open(_home: &Home) -> Self {
        Self
    }

    pub fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>, TokenStoreError> {
        match entry()?.get_password() {
            Ok(text) => Ok(Some(Zeroizing::new(text.into_bytes()))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(TokenStoreError::Keychain(error.to_string())),
        }
    }

    pub fn write(&self, bytes: &[u8]) -> Result<(), TokenStoreError> {
        let text = std::str::from_utf8(bytes).map_err(|_| TokenStoreError::Corrupt)?;
        entry()?
            .set_password(text)
            .map_err(|error| TokenStoreError::Keychain(error.to_string()))
    }
}

fn entry() -> Result<Entry, TokenStoreError> {
    Entry::new(SERVICE, ACCOUNT).map_err(|error| TokenStoreError::Keychain(error.to_string()))
}
