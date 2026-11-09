//! The agent's own folder: its configuration file and, on Linux, its token file (MET-002,
//! MET-013).

use std::fs::DirBuilder;
use std::io;
use std::os::unix::fs::DirBuilderExt as _;
use std::path::{Path, PathBuf};

use crate::config::private_file::{self, PrivateFileError};
use crate::config::{Config, ConfigError};

/// Names the folder; the systemd system unit sets it to its state directory (MET-019).
pub const HOME_VARIABLE: &str = "ANIMA_AGENT_HOME";

const CONFIG_FILE: &str = "agent.json";
const TOKEN_FILE: &str = "token";
const FOLDER: &str = "anima-agent";

/// The folder holding the agent's own files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Home {
    dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum HomeError {
    #[error("no folder for the agent's files: set {HOME_VARIABLE}")]
    NoFolder,
    #[error("the agent's folder: {0}")]
    Io(#[from] io::Error),
    #[error("the agent's configuration file: {0}")]
    File(#[from] PrivateFileError),
    #[error("{0}")]
    Config(#[from] ConfigError),
}

impl Home {
    /// The folder for this computer, from `env` (a lookup of environment variables):
    /// `ANIMA_AGENT_HOME`, else on macOS `~/Library/Application Support/anima-agent`, else
    /// `$XDG_CONFIG_HOME/anima-agent` or `~/.config/anima-agent`.
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

    /// The token file of the Linux token store.
    #[must_use]
    pub fn token_path(&self) -> PathBuf {
        self.dir.join(TOKEN_FILE)
    }

    /// Makes the folder (mode 0700) when it is missing.
    ///
    /// # Errors
    /// [`HomeError::Io`] when it cannot be made.
    pub fn ensure(&self) -> Result<(), HomeError> {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)?;
        Ok(())
    }

    /// The configuration file; `None` when the agent was never enrolled.
    ///
    /// # Errors
    /// [`HomeError`] when the file cannot be read or does not parse.
    pub fn read_config(&self) -> Result<Option<Config>, HomeError> {
        match std::fs::read(self.dir.join(CONFIG_FILE)) {
            Ok(bytes) => Ok(Some(Config::parse(&bytes)?)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
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
        let set = Home::locate(env(&[(HOME_VARIABLE, "/var/lib/anima-agent")]), false);
        assert_eq!(set.expect("set").dir(), Path::new("/var/lib/anima-agent"));
        let xdg = Home::locate(env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")]), false);
        assert_eq!(xdg.expect("xdg").dir(), Path::new("/x/anima-agent"));
        let linux = Home::locate(env(&[("HOME", "/h")]), false);
        assert_eq!(
            linux.expect("linux").dir(),
            Path::new("/h/.config/anima-agent")
        );
        let mac = Home::locate(env(&[("HOME", "/Users/a")]), true);
        assert_eq!(
            mac.expect("mac").dir(),
            Path::new("/Users/a/Library/Application Support/anima-agent")
        );
        assert!(matches!(
            Home::locate(env(&[]), false),
            Err(HomeError::NoFolder)
        ));
    }
}
