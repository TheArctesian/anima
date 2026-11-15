//! Where one bundled PostgreSQL keeps its files. Pure: paths in, paths out; nothing is read,
//! created or checked on disk.
//!
//! ```text
//! <root>/cluster/      the PostgreSQL data folder
//! <root>/urls/         owner.url and hub.url; the hub's ANIMA_DATABASE_URL_FILE names hub.url
//! <root>/server.log    the server's own log
//! <root>/lock          held by the one copy that runs this database
//! <socket folder>/     the Unix socket; the server has no TCP listener (DB-001)
//! ```

use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// PostgreSQL names its socket file after the port, also when it listens on no port.
const SOCKET_FILE: &str = ".s.PGSQL.5432";

/// The most bytes a Unix socket path can have on macOS, whose `sun_path` holds 104 with the
/// final NUL. Linux holds 4 more; one limit for both keeps a root valid on either.
const SOCKET_PATH_LIMIT: usize = 103;

/// The application's identifier, which names its folder in `~/Library/Application Support`.
const MACOS_APPLICATION: &str = "ing.anima.desktop";

/// The operating systems the application hub runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Linux,
}

impl Platform {
    /// The platform this program was built for.
    pub const HOST: Self = if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Linux
    };
}

/// The folders of the user who runs the hub, as the environment gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFolders {
    pub home: PathBuf,
    /// `XDG_DATA_HOME`. Linux only.
    pub xdg_data_home: Option<PathBuf>,
    /// `XDG_RUNTIME_DIR`. Linux only.
    pub xdg_runtime_dir: Option<PathBuf>,
}

/// The two logins that get a URL file. The hub is given the owner's file for `migrate` and
/// its own to serve (K2 "Roles", R-016).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Applies migrations and owns every object.
    Owner,
    /// The hub's runtime login, which owns nothing.
    Hub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    #[error("the {folder} is not an absolute path")]
    NotAbsolute { folder: &'static str },
    #[error("the {folder}'s path has a control character")]
    ControlCharacter { folder: &'static str },
    #[error(
        "the database socket path is {length} bytes and the limit is {limit}; \
         the home folder path is too long"
    )]
    SocketPathTooLong { length: usize, limit: usize },
    #[error(
        "the socket folder's path has a comma, a quotation mark or a space at an end, or is \
         not UTF-8; PostgreSQL would read another folder from it"
    )]
    SocketFolderList,
}

/// Every path of one bundled database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    root: PathBuf,
    socket_folder: PathBuf,
}

impl Layout {
    /// A layout with its files below `root` and its socket in `socket_folder`.
    ///
    /// # Errors
    /// When a folder is not absolute, or the socket file's path is longer than a Unix socket
    /// path can be.
    pub fn new(root: PathBuf, socket_folder: PathBuf) -> Result<Self, LayoutError> {
        absolute(&root, "database root")?;
        absolute(&socket_folder, "socket folder")?;
        // PostgreSQL writes both paths as lines of `postmaster.pid`; a line break in one
        // would move the lines that tell if the server is ready.
        for (folder, name) in [(&root, "database root"), (&socket_folder, "socket folder")] {
            let has_control = folder
                .as_os_str()
                .as_encoded_bytes()
                .iter()
                .any(u8::is_ascii_control);
            if has_control {
                return Err(LayoutError::ControlCharacter { folder: name });
            }
        }
        // `unix_socket_directories` is a list: a comma would split the path, and PostgreSQL
        // trims spaces at both ends of an entry. Either gives a folder that was never checked.
        let listed = socket_folder
            .to_str()
            .is_none_or(|text| text.contains([',', '"']) || text.trim() != text);
        if listed {
            return Err(LayoutError::SocketFolderList);
        }
        let layout = Self {
            root,
            socket_folder,
        };
        let length = layout.socket_file().as_os_str().as_encoded_bytes().len();
        if length > SOCKET_PATH_LIMIT {
            return Err(LayoutError::SocketPathTooLong {
                length,
                limit: SOCKET_PATH_LIMIT,
            });
        }
        Ok(layout)
    }

    /// The layout the application uses for `user` on `platform`.
    ///
    /// macOS keeps the socket beside the root in a folder with a short name, because
    /// `Application Support` already takes much of the socket path limit. Linux keeps it in
    /// the runtime folder, which the system empties at sign-out.
    ///
    /// # Errors
    /// As [`Layout::new`]; the home folder is the path that can be too long.
    pub fn for_user(platform: Platform, user: &UserFolders) -> Result<Self, LayoutError> {
        absolute(&user.home, "home folder")?;
        match platform {
            Platform::MacOs => {
                let application = user
                    .home
                    .join("Library/Application Support")
                    .join(MACOS_APPLICATION);
                Self::new(application.join("hub/database"), application.join("pg"))
            }
            Platform::Linux => {
                let share = given(user.xdg_data_home.as_deref())
                    .map_or_else(|| user.home.join(".local/share"), Path::to_path_buf)
                    .join("anima");
                let socket_folder = given(user.xdg_runtime_dir.as_deref())
                    .map_or_else(|| share.join("pg"), |runtime| runtime.join("anima"));
                Self::new(share.join("hub/database"), socket_folder)
            }
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The PostgreSQL data folder. PostgreSQL's word for it is "cluster".
    #[must_use]
    pub fn cluster(&self) -> PathBuf {
        self.root.join("cluster")
    }

    /// Where a new cluster is made before it takes its place. A folder here never held data,
    /// so one that a crash left is removed.
    #[must_use]
    pub fn cluster_being_made(&self) -> PathBuf {
        self.root.join("cluster.new")
    }

    /// The folder of the URL files. Each file in it holds a password.
    #[must_use]
    pub fn urls_folder(&self) -> PathBuf {
        self.root.join("urls")
    }

    /// The file that holds `role`'s connection URL.
    #[must_use]
    pub fn url_file(&self, role: Role) -> PathBuf {
        self.urls_folder().join(match role {
            Role::Owner => "owner.url",
            Role::Hub => "hub.url",
        })
    }

    #[must_use]
    pub fn server_log(&self) -> PathBuf {
        self.root.join("server.log")
    }

    #[must_use]
    pub fn lock_file(&self) -> PathBuf {
        self.root.join("lock")
    }

    #[must_use]
    pub fn socket_folder(&self) -> &Path {
        &self.socket_folder
    }

    #[must_use]
    pub fn socket_file(&self) -> PathBuf {
        self.socket_folder.join(SOCKET_FILE)
    }
}

fn absolute(path: &Path, folder: &'static str) -> Result<(), LayoutError> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(LayoutError::NotAbsolute { folder })
    }
}

/// An XDG folder counts only when it is absolute; the specification says to ignore any other
/// value, an empty one included.
fn given(folder: Option<&Path>) -> Option<&Path> {
    folder.filter(|path| path.is_absolute())
}
