//! Where the hub on this computer keeps its files. Pure: paths in, paths out.
//!
//! ```text
//! <root>/database/           the bundled PostgreSQL (S3)
//! <root>/programs/current/   the programs the unit runs
//! <root>/programs/next/      a copy in progress; never run
//! <root>/vault.key           the hub's ANIMA_KEY_FILE
//! <root>/log/                one log for each member of the group
//! <root>/link/               the state of `anima-link hub`: its key, devices, control socket
//! <root>/lock                held by the one `run` of this root
//! <root>/state               what `run` last reported
//! ```

use std::path::{Path, PathBuf};

use anima_database::{Layout, LayoutError, Platform, UserFolders};

#[cfg(test)]
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

    #[must_use]
    pub fn state_file(&self) -> PathBuf {
        self.root.join("state")
    }
}
