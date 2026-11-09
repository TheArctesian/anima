//! Where the agent keeps its token: the macOS Keychain, or on Linux a private file (MET-013,
//! K8 Section 2). The record is the `Renewed` body, so the expiry is stored with the token.

use agent_core::wire::{AgentToken, Renewed};
use chrono::{DateTime, Utc};
use zeroize::Zeroizing;

use crate::config::home::Home;

#[cfg(not(target_os = "macos"))]
mod file;
#[cfg(target_os = "macos")]
mod keychain;

#[cfg(not(target_os = "macos"))]
use file::Backend;
#[cfg(target_os = "macos")]
use keychain::Backend;

/// The agent token and when it expires. Neither `Debug` nor `Clone`, as the token is not.
pub struct Credential {
    pub token: AgentToken,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum TokenStoreError {
    #[error("the token file: {0}")]
    File(#[from] crate::config::private_file::PrivateFileError),
    #[error("the stored token record does not read")]
    Corrupt,
    #[error("the agent's folder: {0}")]
    Folder(#[from] crate::config::home::HomeError),
    #[cfg(target_os = "macos")]
    #[error("the Keychain: {0}")]
    Keychain(String),
}

/// The agent's token store for this platform.
pub struct TokenStore {
    backend: Backend,
}

impl From<Renewed> for Credential {
    fn from(renewed: Renewed) -> Self {
        Self {
            token: renewed.token,
            expires_at: renewed.expires_at,
        }
    }
}

impl Credential {
    fn render(self) -> Zeroizing<Vec<u8>> {
        let record = Renewed {
            token: self.token,
            expires_at: self.expires_at,
        };
        Zeroizing::new(serde_json::to_vec(&record).expect("a token record serialises"))
    }
}

impl TokenStore {
    #[must_use]
    pub fn open(home: &Home) -> Self {
        Self {
            backend: Backend::open(home),
        }
    }

    /// The stored credential; `None` when the agent holds none.
    ///
    /// # Errors
    /// [`TokenStoreError`] when the store cannot be read, is exposed, or does not parse.
    pub fn load(&self) -> Result<Option<Credential>, TokenStoreError> {
        let Some(bytes) = self.backend.read()? else {
            return Ok(None);
        };
        let record: Renewed =
            serde_json::from_slice(&bytes).map_err(|_| TokenStoreError::Corrupt)?;
        Ok(Some(record.into()))
    }

    /// Replaces the stored credential. The old one stays when this fails (K8 Section 5).
    ///
    /// # Errors
    /// [`TokenStoreError`] when it cannot be written.
    pub fn save(&self, credential: &Credential) -> Result<(), TokenStoreError> {
        let token = AgentToken::parse(credential.token.expose()).ok_or(TokenStoreError::Corrupt)?;
        let copy = Credential {
            token,
            expires_at: credential.expires_at,
        };
        self.backend.write(&copy.render())
    }
}
