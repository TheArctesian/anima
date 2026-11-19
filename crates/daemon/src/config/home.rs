//! The daemon's own folder: its configuration file and, on Linux, its token file (MET-002,
//! MET-013).

use std::fs::DirBuilder;
use std::io;
use std::os::unix::fs::DirBuilderExt as _;
use std::path::{Path, PathBuf};

use crate::config::private_file::{self, PrivateFileError};
use crate::config::{Config, ConfigError};

/// Names the folder; the systemd system unit sets it to its state directory (MET-019).
pub const HOME_VARIABLE: &str = "ANIMA_DAEMON_HOME";

const CONFIG_FILE: &str = "daemon.json";
const TOKEN_FILE: &str = "token";
const FOLDER: &str = "anima-daemon";
const LINK_SOCKET: &str = "link.sock";

/// The folder holding the daemon's own files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Home {
    dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum HomeError {
    #[error("no folder for the daemon's files: set {HOME_VARIABLE}")]
    NoFolder,
    #[error("the daemon's folder: {0}")]
    Io(#[from] io::Error),
    #[error(
        "the daemon's folder is not a real folder owned by this user with no access for others, or sits under a folder others can write to"
    )]
    Exposed,
    #[error("the daemon's configuration file: {0}")]
    File(#[from] PrivateFileError),
    #[error("{0}")]
    Config(#[from] ConfigError),
}

impl Home {
    /// The folder for this computer, from `env` (a lookup of environment variables):
    /// `ANIMA_DAEMON_HOME`, else on macOS `~/Library/Application Support/anima-daemon`, else
    /// `$XDG_CONFIG_HOME/anima-daemon` or `~/.config/anima-daemon`.
    ///
    /// # Errors
    /// [`HomeError::NoFolder`] when none of the variables is set.
    pub fn locate(env: impl Fn(&str) -> Option<String>, macos: bool) -> Result<Self, HomeError> {
        let dir = if let Some(set) = env(HOME_VARIABLE) {
            PathBuf::from(set)
        } else if macos {
            let home = env("HOME").ok_or(HomeError::NoFolder)?;
            Path::new(&home)
                .join("Library/Application Support")
                .join(FOLDER)
        } else if let Some(config) = env("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
            Path::new(&config).join(FOLDER)
        } else {
            let home = env("HOME").ok_or(HomeError::NoFolder)?;
            Path::new(&home).join(".config").join(FOLDER)
        };
        Ok(Self { dir })
    }

    /// The folder at `dir`, for tests and for a caller that already chose.
    #[must_use]
    pub const fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The socket that `anima-link device` makes for a daemon that was enrolled with an old
    /// link origin (K8 Section 1).
    #[must_use]
    pub fn link_socket(&self) -> PathBuf {
        self.dir.join(LINK_SOCKET)
    }

    /// The token file of the Linux token store.
    #[must_use]
    pub fn token_path(&self) -> PathBuf {
        self.dir.join(TOKEN_FILE)
    }

    /// Makes the folder (mode 0700) when it is missing, and refuses one that is a link, is not
    /// owned by the user running the daemon, or gives group or others any access: files in it
    /// decide where the bearer goes. Each folder above it (as the path stands, not
    /// with links resolved) is refused when others can write to it without the sticky bit, since
    /// another user could replace the daemon's folder there. Not checked: the owner of those
    /// folders, whether the group can write to them (a private group per user is common), and
    /// links in the path. This is weaker than the OpenSSH `StrictModes` rule, which refuses
    /// group-writable folders too and would refuse homes made with `umask 002`.
    ///
    /// # Errors
    /// [`HomeError::Exposed`] for such a folder, [`HomeError::Io`] when it cannot be made.
    pub fn ensure(&self) -> Result<(), HomeError> {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)?;
        if private_file::folder_private(&self.dir)? {
            Ok(())
        } else {
            Err(HomeError::Exposed)
        }
    }

    /// The configuration file; `None` when the daemon was never enrolled. The folder and the
    /// file are held to the same rules as the token file.
    ///
    /// # Errors
    /// [`HomeError`] when the folder or file is exposed, or the file cannot be read or parsed.
    pub fn read_config(&self) -> Result<Option<Config>, HomeError> {
        self.ensure()?;
        match private_file::read(&self.dir.join(CONFIG_FILE))? {
            Some(bytes) => Ok(Some(Config::parse(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Writes the configuration file by temporary file and rename.
    ///
    /// # Errors
    /// [`HomeError`] when the folder or the file cannot be written.
    pub fn write_config(&self, config: &Config) -> Result<(), HomeError> {
        self.ensure()?;
        private_file::write(&self.dir, CONFIG_FILE, &config.render())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        }
    }

    #[test]
    fn met_002_the_folder_follows_the_variable_then_the_platform() {
        let set = Home::locate(env(&[(HOME_VARIABLE, "/var/lib/anima-daemon")]), false);
        assert_eq!(set.expect("set").dir(), Path::new("/var/lib/anima-daemon"));
        let xdg = Home::locate(env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")]), false);
        assert_eq!(xdg.expect("xdg").dir(), Path::new("/x/anima-daemon"));
        let linux = Home::locate(env(&[("HOME", "/h")]), false);
        assert_eq!(
            linux.expect("linux").dir(),
            Path::new("/h/.config/anima-daemon")
        );
        let mac = Home::locate(env(&[("HOME", "/Users/a")]), true);
        assert_eq!(
            mac.expect("mac").dir(),
            Path::new("/Users/a/Library/Application Support/anima-daemon")
        );
        assert!(matches!(
            Home::locate(env(&[]), false),
            Err(HomeError::NoFolder)
        ));
    }
}
