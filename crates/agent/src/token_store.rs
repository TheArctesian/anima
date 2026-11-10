//! Where the agent keeps its token: the macOS Keychain, or on Linux a private file (MET-013,
//! K8 Section 2). The record is the `Renewed` body with the hub origin added, so the expiry and
//! the hub are stored with the token.

use agent_core::wire::{AgentToken, Renewed};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::config::home::Home;
use crate::config::origin::Origin;

#[cfg(not(target_os = "macos"))]
mod file;
#[cfg(target_os = "macos")]
mod keychain;

#[cfg(not(target_os = "macos"))]
use file::Backend;
#[cfg(target_os = "macos")]
use keychain::Backend;

/// The latest expiry the agent keeps: a token lives 365 days (MET-013), so a later date from
/// the hub is capped here and the agent renews on its own schedule.
pub const MAX_LIFE: TimeDelta = TimeDelta::days(366);

/// The agent token, when it expires, and the hub it was issued by. Neither `Debug` nor `Clone`,
/// as the token is not.
pub struct Credential {
    pub token: AgentToken,
    pub expires_at: DateTime<Utc>,
    /// The hub origin kept beside the token: the agent refuses to send the token to another
    /// (on macOS the Keychain record is the one place an attacker with file access cannot edit).
    pub hub: Origin,
}

/// The hub as the record names it; a record without one does not read.
#[derive(Deserialize)]
struct HubField {
    hub: String,
}

/// The stored record: the `Renewed` body with the hub beside it.
#[derive(Serialize)]
struct Record<'a> {
    #[serde(flatten)]
    renewed: Renewed,
    hub: &'a str,
}

/// Bytes a record needs besides the hub's text: the token, its key, the expiry and the
/// punctuation come to under 150; the hub's text may grow to 6 bytes a character when escaped.
fn record_room(hub: &str) -> usize {
    256 + 6 * hub.len()
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

impl Credential {
    /// A token the hub just gave: its expiry is capped at `now` plus [`MAX_LIFE`].
    #[must_use]
    pub fn received(
        token: AgentToken,
        expires_at: DateTime<Utc>,
        hub: Origin,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            token,
            expires_at: expires_at.min(now + MAX_LIFE),
            hub,
        }
    }

    /// The credential a stored record holds. The error says only that it does not read: the
    /// parser's message could quote the token (LOG-007).
    ///
    /// # Errors
    /// [`TokenStoreError::Corrupt`] for a record that does not read.
    pub fn from_record(bytes: &[u8]) -> Result<Self, TokenStoreError> {
        let record: Renewed =
            serde_json::from_slice(bytes).map_err(|_| TokenStoreError::Corrupt)?;
        let field: HubField =
            serde_json::from_slice(bytes).map_err(|_| TokenStoreError::Corrupt)?;
        let hub = Origin::parse(&field.hub).map_err(|_| TokenStoreError::Corrupt)?;
        Ok(Self {
            token: record.token,
            expires_at: record.expires_at,
            hub,
        })
    }

    /// The record: the `Renewed` body with the hub origin added. The buffer has room for it
    /// from the start, so it never moves and its wipe on drop covers the token.
    fn render(self) -> Zeroizing<Vec<u8>> {
        let hub = self.hub.as_text();
        let record = Record {
            renewed: Renewed {
                token: self.token,
                expires_at: self.expires_at,
            },
            hub: &hub,
        };
        let mut bytes = Zeroizing::new(Vec::with_capacity(record_room(&hub)));
        serde_json::to_writer(&mut *bytes, &record).expect("a token record serialises");
        bytes
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
        Credential::from_record(&bytes).map(Some)
    }

    /// Replaces the stored credential after a renewal. The old one stays when this fails
    /// (K8 Section 5).
    ///
    /// # Errors
    /// [`TokenStoreError`] when it cannot be written.
    pub fn save(&self, credential: &Credential) -> Result<(), TokenStoreError> {
        self.write(credential, false)
    }

    /// Stores the credential of a new enrolment; the Keychain item is made afresh.
    ///
    /// # Errors
    /// [`TokenStoreError`] when it cannot be written.
    pub fn save_new(&self, credential: &Credential) -> Result<(), TokenStoreError> {
        self.write(credential, true)
    }

    fn write(&self, credential: &Credential, fresh: bool) -> Result<(), TokenStoreError> {
        let token = AgentToken::parse(credential.token.expose()).ok_or(TokenStoreError::Corrupt)?;
        let copy = Credential {
            token,
            expires_at: credential.expires_at,
            hub: credential.hub.clone(),
        };
        self.backend.write(&copy.render(), fresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_013_the_record_is_written_into_room_made_at_the_start() {
        let hub = Origin::parse("https://hub.example").expect("origin");
        let credential = Credential {
            token: AgentToken::from_random([3; 32]),
            expires_at: DateTime::UNIX_EPOCH,
            hub: hub.clone(),
        };
        let room = Vec::<u8>::with_capacity(record_room(&hub.as_text())).capacity();
        let bytes = credential.render();
        assert_eq!(
            bytes.capacity(),
            room,
            "the buffer did not grow, so it did not move"
        );
        assert!(bytes.starts_with(b"{\"token\":"));
        assert!(bytes.ends_with(b"\"hub\":\"https://hub.example\"}"));
    }
}
