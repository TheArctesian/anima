//! Where the daemon keeps its key cache: where it keeps its token (K-E5 Section 2.1). The macOS
//! Keychain, or on Linux the file `keys` in the daemon's folder, mode 0600, written to a
//! temporary file and renamed. An empty cache is stored as no record at all.

use super::cache::KeyCache;
use super::record::{self, RECORD_LIMIT};
use crate::config::home::Home;

#[cfg(not(target_os = "macos"))]
mod file;
#[cfg(target_os = "macos")]
mod keychain;

#[cfg(not(target_os = "macos"))]
use file::Backend;
#[cfg(target_os = "macos")]
use keychain::Backend;

/// Why the key store failed. No message holds a value or a key name (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum KeyStoreError {
    #[error("the key cache file: {0}")]
    File(#[from] crate::config::private_file::PrivateFileError),
    #[error("the daemon's folder: {0}")]
    Folder(#[from] crate::config::home::HomeError),
    #[cfg(target_os = "macos")]
    #[error("the Keychain: {0}")]
    Keychain(String),
}

/// The daemon's key store for this platform.
pub struct KeyStore {
    backend: Backend,
}

impl KeyStore {
    #[must_use]
    pub fn open(home: &Home) -> Self {
        Self {
            backend: Backend::open(home),
        }
    }

    /// The stored cache; an empty one when none is stored. A record that does not read is
    /// deleted and read as empty, so a damaged record is never used.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the store cannot be read, is exposed, or cannot be cleared.
    pub fn load(&self) -> Result<KeyCache, KeyStoreError> {
        let Some(bytes) = self.backend.read(RECORD_LIMIT)? else {
            return Ok(KeyCache::default());
        };
        if let Some(cache) = record::parse(&bytes) {
            return Ok(cache);
        }
        self.backend.delete()?;
        Ok(KeyCache::default())
    }

    /// Replaces the stored cache with `cache`; deletes the record when `cache` is empty.
    ///
    /// # Errors
    /// [`KeyStoreError`] when it cannot be written.
    pub fn save(&self, cache: &KeyCache) -> Result<(), KeyStoreError> {
        if cache.is_empty() {
            return self.backend.delete();
        }
        self.backend.write(&record::render(cache))
    }

    /// Deletes the whole cache.
    ///
    /// # Errors
    /// [`KeyStoreError`] when it cannot be deleted.
    pub fn forget(&self) -> Result<(), KeyStoreError> {
        self.backend.delete()
    }
}

/// The file store only: the Keychain store is checked by a person (HT-035).
#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use chrono::DateTime;
    use daemon_core::wire::{FetchedKey, KeyValue};
    use zeroize::Zeroizing;

    use super::*;

    #[test]
    fn r_147_the_cache_is_a_0600_file_beside_the_token_and_forget_removes_it() {
        let dir = tempfile::tempdir().unwrap();
        let home = Home::at(dir.path().join("daemon"));
        let store = KeyStore::open(&home);
        let key = FetchedKey {
            name: "cloudflare.dns".to_owned(),
            version: 2,
            value: KeyValue::new(Zeroizing::new(b"value".to_vec())).unwrap(),
        };
        let cache = KeyCache::of(vec![key], Some(DateTime::UNIX_EPOCH));
        store.save(&cache).unwrap();

        let path = home.dir().join("keys");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let read = store.load().unwrap();
        assert_eq!(read.value("cloudflare.dns").unwrap().expose(), b"value");

        store.forget().unwrap();
        assert!(!path.exists());
        assert!(store.load().unwrap().is_empty());
    }
}
