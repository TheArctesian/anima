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

/// Marks this process not dumpable (D-19): the kernel then writes no core dump of it, so a build
/// step that crashes the app cannot read the opened key from the dump, and no process without
/// `CAP_SYS_PTRACE` may attach to it or read its memory. Call once at startup, before a key is
/// opened. Linux only; `builder_tools_check` reads the state back and the row warns if it failed.
#[cfg(target_os = "linux")]
pub fn keep_out_of_core_dumps() -> nix::Result<()> {
    nix::sys::prctl::set_dumpable(false)
}

/// Whether this process may be dumped; on systems other than Linux, as if it may (D-19).
pub fn dumpable() -> bool {
    #[cfg(target_os = "linux")]
    return nix::sys::prctl::get_dumpable().unwrap_or(true);
    #[cfg(not(target_os = "linux"))]
    true
}
