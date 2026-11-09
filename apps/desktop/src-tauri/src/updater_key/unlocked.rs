//! The updater key opened for this app session (D-19): held only in this process's memory, in a
//! `Zeroizing` seed, and dropped when the operator locks it, turns building off or quits. The
//! lock is held only for short synchronous sections, never across an `await`.

use std::sync::{Mutex, PoisonError};

use super::{
    error::UpdaterKeyError,
    secret::SecretKey,
    signature::{self, Signed},
};

/// The opened key, if the operator unlocked it in this session.
#[derive(Default)]
pub struct UnlockedKey {
    key: Mutex<Option<SecretKey>>,
}

impl UnlockedKey {
    fn with_key<T>(&self, use_key: impl FnOnce(&mut Option<SecretKey>) -> T) -> T {
        use_key(&mut self.key.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Keeps `key` open; an earlier key is dropped and wiped.
    pub(crate) fn set(&self, key: SecretKey) {
        self.with_key(|slot| *slot = Some(key));
    }

    /// Drops the opened key; its seed is wiped.
    pub fn lock(&self) {
        self.with_key(|slot| *slot = None);
    }

    pub fn is_unlocked(&self) -> bool {
        self.with_key(|slot| slot.is_some())
    }

    /// The `.sig` text for the file whose prehash is `digest` (BLD-045, BLD-046); `Locked` when
    /// the key is not open. The key never leaves this process.
    pub(crate) fn sign(
        &self,
        digest: &[u8; 64],
        signed: &Signed<'_>,
    ) -> Result<String, UpdaterKeyError> {
        self.with_key(|slot| {
            let key = slot.as_ref().ok_or(UpdaterKeyError::Locked)?;
            signature::sign_prehashed(key, digest, signed)
        })
    }
}
