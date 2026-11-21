//! The daemon in the container of a hosted hub (HMD-071, task HM-T35 of hosted-mode.md): where
//! it keeps its token, how it reaches the hub and what it measures. Pure.
//!
//! The hub serves it the routes of K8 on a private socket in the socket folder of the database
//! (this user's, mode 0700). When the daemon has no token yet, the hub writes one enrolment
//! code of its own device to a file beside that socket at its start; `run` gives the code to
//! `anima-daemon enrol -` on standard input and removes the file. A daemon that holds its token
//! and its configuration is not enrolled again: no new code, token or device at each start.
//! The daemon deletes a token that the hub refuses (HM-T35), so the next start enrols again.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{Carried, Pool};
use crate::Folders;
use crate::child::Spec;

/// The daemon of the hub's own device. A Rust program; the pool's image may leave it out.
pub const DAEMON: &str = "anima-daemon";

/// The files of an enrolment in the daemon's folder (`crates/daemon/src/config/home.rs`): the
/// token, and the configuration that names the hub and the device.
const ENROLMENT_FILES: [&str; 2] = ["token", "daemon.json"];

/// The daemon of a hosted hub: its program, its folder, the hub's socket for it, the file of its
/// code and the folder whose bytes it measures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnDaemon {
    program: PathBuf,
    home: PathBuf,
    socket: PathBuf,
    code_file: PathBuf,
    measured: PathBuf,
}

impl OwnDaemon {
    /// The daemon of the hub of `folders`, or `None` when the hub is not hosted (`pool`) or the
    /// folder of the programs has no daemon (`present`).
    #[must_use]
    pub fn of(carried: &Carried, folders: &Folders, pool: &Pool, present: bool) -> Option<Self> {
        pool.cloud_key_file.as_ref()?;
        present.then(|| Self {
            program: carried.program(DAEMON),
            home: folders.root().join("daemon"),
            socket: folders.database().socket_folder().join("own-daemon.sock"),
            code_file: folders.database().socket_folder().join("own-daemon.code"),
            measured: folders.root().to_path_buf(),
        })
    }

    /// Whether the daemon holds an enrolment: its token and its configuration are both there.
    /// An enrolment that stopped between the two files counts as none.
    #[must_use]
    pub fn is_enrolled(&self) -> bool {
        ENROLMENT_FILES
            .iter()
            .all(|file| self.home.join(file).is_file())
    }

    /// The file in which the hub writes the code of an enrolment.
    #[must_use]
    pub fn code_file(&self) -> &Path {
        &self.code_file
    }

    /// What the hub is told: the socket, and the code file when the daemon has no token.
    #[must_use]
    pub fn hub_environment(&self, enrolled: bool) -> Vec<(&'static str, OsString)> {
        let mut env = vec![("ANIMA_OWN_DAEMON_SOCKET", self.socket.clone().into())];
        if !enrolled {
            env.push(("ANIMA_OWN_DAEMON_CODE_FILE", self.code_file.clone().into()));
        }
        env
    }

    /// `anima-daemon enrol - --hub unix:<socket>`: the code comes on standard input.
    #[must_use]
    pub fn enrolment(&self) -> Spec {
        let mut hub = OsString::from("unix:");
        hub.push(&self.socket);
        Spec {
            name: "anima-daemon enrol",
            program: self.program.clone(),
            args: vec!["enrol".into(), "-".into(), "--hub".into(), hub],
            env: self.environment(),
        }
    }

    /// `anima-daemon run --hosted-hub <hub root>`: the container's figures and the bytes of the
    /// hub's folder, nothing of the server (HMD-071, HMD-072, HMD-074).
    #[must_use]
    pub fn collection(&self) -> Spec {
        Spec {
            name: DAEMON,
            program: self.program.clone(),
            args: vec![
                "run".into(),
                "--hosted-hub".into(),
                self.measured.clone().into(),
            ],
            env: self.environment(),
        }
    }

    fn environment(&self) -> Vec<(&'static str, OsString)> {
        vec![("ANIMA_DAEMON_HOME", self.home.clone().into())]
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn own_in(root: &Path) -> OwnDaemon {
        let hosted = Pool {
            cloud_key_file: Some(PathBuf::from("/etc/anima/cloud.key")),
            ..Pool::default()
        };
        let folders = Folders::new(root.join("hub"), root.join("s")).unwrap();
        let carried = Carried::installed_in(&root.join("programs"));
        OwnDaemon::of(&carried, &folders, &hosted, true).unwrap()
    }

    #[test]
    fn hm_t35_only_a_token_with_its_configuration_spares_the_next_start_an_enrolment() {
        let folder = tempfile::tempdir().unwrap();
        let own = own_in(folder.path());
        let home = folder.path().join("hub/daemon");
        std::fs::create_dir_all(&home).unwrap();
        assert!(!own.is_enrolled(), "a new hub enrols its daemon");
        std::fs::write(home.join("token"), "t").unwrap();
        assert!(
            !own.is_enrolled(),
            "an enrolment cut before its configuration"
        );
        std::fs::write(home.join("daemon.json"), "{}").unwrap();
        assert!(own.is_enrolled(), "a restart keeps the token");
        std::fs::remove_file(home.join("token")).unwrap();
        assert!(
            !own.is_enrolled(),
            "a token that the daemon deleted on a refusal"
        );
    }
}
