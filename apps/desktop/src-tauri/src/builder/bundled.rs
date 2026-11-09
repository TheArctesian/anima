//! The updater file the last bundle step made (D-13): its job, its path and the hash of its
//! bytes. Signing and checking take the file from here, so they sign only what this bundle step
//! wrote, and only while its bytes are unchanged. The next checkout forgets it. The lock is held
//! only for short synchronous sections.

use std::{
    path::PathBuf,
    sync::{Mutex, PoisonError},
};

use super::job::{Commit, Target, Version};

/// The bundle of one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub commit: Commit,
    pub target: Target,
    pub version: Version,
    pub path: PathBuf,
    /// BLAKE2b-512 of the file, the hash minisign signs.
    pub digest: [u8; 64],
}

/// The last bundle, until the next checkout.
#[derive(Debug, Default)]
pub struct Bundled {
    last: Mutex<Option<Bundle>>,
}

impl Bundled {
    fn with_last<T>(&self, change: impl FnOnce(&mut Option<Bundle>) -> T) -> T {
        change(&mut self.last.lock().unwrap_or_else(PoisonError::into_inner))
    }

    pub fn record(&self, bundle: Bundle) {
        self.with_last(|last| *last = Some(bundle));
    }

    pub fn forget(&self) {
        self.with_last(|last| *last = None);
    }

    /// The bundle of `target` at `version`, when that is the last one made.
    pub fn of(&self, target: Target, version: &Version) -> Option<Bundle> {
        self.with_last(|last| {
            last.as_ref()
                .filter(|bundle| bundle.target == target && &bundle.version == version)
                .cloned()
        })
    }
}
