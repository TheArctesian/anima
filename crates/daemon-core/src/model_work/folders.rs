//! The folders that model work writes (Section 10, CAG-009, MET-002): pure, from the kind of
//! the installation and the environment.

use std::path::{Path, PathBuf};

use uuid::Uuid;

use super::names::Word;
use super::releases::Release;

const FOLDER: &str = "anima-daemon";

/// How the daemon is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Installation {
    /// A launchd agent of the user on macOS.
    MacUser,
    /// A systemd user unit on Linux.
    LinuxUser,
    /// A systemd system unit on Linux.
    LinuxSystem,
}

/// The three folders of model work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folders {
    pub runtimes: PathBuf,
    pub models: PathBuf,
    pub state: PathBuf,
}

fn absolute(value: Option<String>) -> Option<PathBuf> {
    value
        .filter(|value| Path::new(value).is_absolute())
        .map(PathBuf::from)
}

/// The folders for `installation`, from `env` (a lookup of environment variables). With
/// `ANIMA_DAEMON_HOME` set, the three folders are below it. `None` when a variable that the
/// layout needs is not set to an absolute path.
#[must_use]
pub fn layout(installation: Installation, env: impl Fn(&str) -> Option<String>) -> Option<Folders> {
    let below = |base: &Path, state: &Path| Folders {
        runtimes: base.join("runtimes"),
        models: base.join("models"),
        state: state.join("model-state"),
    };
    if let Some(home) = absolute(env("ANIMA_DAEMON_HOME")) {
        return Some(below(&home, &home));
    }
    match installation {
        Installation::MacUser => {
            let base = absolute(env("HOME"))?
                .join("Library/Application Support")
                .join(FOLDER);
            Some(below(&base, &base))
        }
        Installation::LinuxUser => {
            let home = absolute(env("HOME"));
            let data = absolute(env("XDG_DATA_HOME"))
                .or_else(|| Some(home.clone()?.join(".local/share")))?;
            let state =
                absolute(env("XDG_STATE_HOME")).or_else(|| Some(home?.join(".local/state")))?;
            Some(below(&data.join(FOLDER), &state.join(FOLDER)))
        }
        Installation::LinuxSystem => {
            let base = PathBuf::from("/var/lib").join(FOLDER);
            Some(below(&base, &base))
        }
    }
}

impl Folders {
    /// The folder of an installed release: `runtimes/<runtime>/<version>-<backend>/`.
    #[must_use]
    pub fn release(&self, release: &Release) -> PathBuf {
        self.runtimes
            .join(release.runtime.as_str())
            .join(release.folder())
    }

    /// The folder of one server in the state folder: its temporary folder and its output ring.
    #[must_use]
    pub fn server(&self, server: Uuid) -> PathBuf {
        self.state.join("servers").join(server.to_string())
    }
}
