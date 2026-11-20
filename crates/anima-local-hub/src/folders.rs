//! Where the hub on this computer keeps its files. Pure: paths in, paths out.
//!
//! ```text
//! <root>/database/           the bundled PostgreSQL (S3)
//! <root>/programs/current/   the programs the unit runs
//! <root>/programs/next/      a copy in progress; never run
//! <root>/vault.key           the hub's ANIMA_KEY_FILE
//! <root>/data-keys/          the hub's ANIMA_DATA_KEY_DIR: one data key for each extension
//! <root>/files/              the hub's ANIMA_FILES_DIR: the sealed pieces of the files
//! <root>/log/                one log for each member of the group
//! <root>/link/               the state of `anima-link hub`: its key, devices, control socket
//! <root>/lock                held by the one `run` of this root
//! <root>/state               what `run` last reported
//! <root>/change.lock         held while `install`, `update`, `remove`, `start` or `stop` runs
//! <root>/listen              the address that `install --listen` gave
//! <root>/stopped-by-operator the operator asked for a stop; no update starts the hub
//! ```

use std::path::{Path, PathBuf};

use anima_database::{Layout, LayoutError, Platform, UserFolders};

#[cfg(all(test, unix))]
mod tests;

/// The folder of the database below the hub root; S3 puts its root there for a user.
const DATABASE: &str = "database";

/// The hub root and the database's own layout below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folders {
    root: PathBuf,
    database: Layout,
}

impl Folders {
    /// The folders below `root`, with the database's Unix socket in `socket_folder`.
    ///
    /// # Errors
    /// When the database refuses one of the two paths ([`LayoutError`]).
    pub fn new(root: PathBuf, socket_folder: PathBuf) -> Result<Self, LayoutError> {
        let database = Layout::new(root.join(DATABASE), socket_folder)?;
        Ok(Self { root, database })
    }

    /// The folders of the user who runs the hub: the parent of the database root that S3
    /// gives this user.
    ///
    /// # Errors
    /// When the database refuses the user's folders ([`LayoutError`]).
    pub fn for_user(platform: Platform, user: &UserFolders) -> Result<Self, LayoutError> {
        let database = Layout::for_user(platform, user)?;
        let root = database
            .root()
            .parent()
            .map_or_else(|| database.root().to_path_buf(), Path::to_path_buf);
        Ok(Self { root, database })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub const fn database(&self) -> &Layout {
        &self.database
    }

    /// The programs the unit runs. `install` replaces this folder only while nothing runs.
    #[must_use]
    pub fn programs(&self) -> PathBuf {
        self.root.join("programs/current")
    }

    /// Where `install` copies the programs before they take their place.
    #[must_use]
    pub fn programs_being_copied(&self) -> PathBuf {
        self.root.join("programs/next")
    }

    /// Where the programs before an update wait to be removed.
    #[must_use]
    pub fn programs_replaced(&self) -> PathBuf {
        self.root.join("programs/previous")
    }

    #[must_use]
    pub fn all_programs(&self) -> PathBuf {
        self.root.join("programs")
    }

    /// The hub's `ANIMA_KEY_FILE`. It holds a secret.
    #[must_use]
    pub fn vault_key(&self) -> PathBuf {
        self.root.join("vault.key")
    }

    /// The hub's `ANIMA_DATA_KEY_DIR`, beside `vault.key` (K-E5 Section 1). It holds secrets;
    /// the hub makes it and its keys.
    #[must_use]
    pub fn data_keys(&self) -> PathBuf {
        self.root.join("data-keys")
    }

    /// The hub's `ANIMA_FILES_DIR`: the sealed pieces of the files (K-E4 Section 9).
    #[must_use]
    pub fn files(&self) -> PathBuf {
        self.root.join("files")
    }

    /// The state folder of `anima-link hub`. It holds the link's key.
    #[must_use]
    pub fn link(&self) -> PathBuf {
        self.root.join("link")
    }

    /// The control socket that `anima-link hub` makes in its state folder.
    #[must_use]
    pub fn link_control(&self) -> PathBuf {
        self.link().join("control.sock")
    }

    /// The hub's socket for the link (S5): in the database's socket folder, which is this
    /// user's, mode 0700, with a short path.
    #[must_use]
    pub fn link_socket(&self) -> PathBuf {
        self.database.socket_folder().join("link.sock")
    }

    #[must_use]
    pub fn logs(&self) -> PathBuf {
        self.root.join("log")
    }

    #[must_use]
    pub fn lock_file(&self) -> PathBuf {
        self.root.join("lock")
    }

    /// Present from `stop` until `start`, `install`, `remove` or the next `run`.
    #[must_use]
    pub fn stopped_by_operator(&self) -> PathBuf {
        self.root.join("stopped-by-operator")
    }

    /// Held by the one change of this root (`install`, `update`, `remove`, `start`, `stop`).
    #[must_use]
    pub fn change_lock(&self) -> PathBuf {
        self.root.join("change.lock")
    }

    /// The address that `install --listen` gave, which the unit file names.
    #[must_use]
    pub fn listen_file(&self) -> PathBuf {
        self.root.join("listen")
    }

    #[must_use]
    pub fn state_file(&self) -> PathBuf {
        self.root.join("state")
    }
}
