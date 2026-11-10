//! A temporary HOME with an enrolled agent and a stand-in binary, and `install` run against it
//! with a recording stand-in for the commands, so no systemd is touched (D-10).

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use anima_agent::config::home::Home;
use anima_agent::config::private_file;
use anima_agent::readers::Platform;
use anima_agent::token_store::{Credential, TokenStore};
use anima_agent::units::install::{Host, InstallError, install};
use anima_agent::units::plan::Target;
use anima_agent::units::step::Step;
use chrono::TimeDelta;
use tempfile::TempDir;

use crate::kit;

pub struct Rig {
    pub dir: TempDir,
    pub program: PathBuf,
    pub home: Home,
    pub uid: u32,
}

/// A HOME whose default agent folder holds a token expiring `expires_in` after `kit::at(0)`
/// (none when `None`), and a stand-in binary. It is in Cargo's own temporary folder, as the
/// units' `PrivateTmp=yes` hides `/tmp` from the agent.
pub fn rig(expires_in: Option<TimeDelta>) -> Rig {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a folder");
    let program = dir.path().join("bin/anima-agent");
    fs::create_dir(dir.path().join("bin")).expect("bin");
    fs::write(&program, b"#!/bin/true\n").expect("binary");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).expect("chmod");
    let home = Home::at(dir.path().join(".config/anima-agent"));
    if let Some(expires_in) = expires_in {
        enrol(&home, expires_in);
    }
    let uid = private_file::running_user().expect("a uid");
    Rig {
        dir,
        program,
        home,
        uid,
    }
}

pub fn enrol(home: &Home, expires_in: TimeDelta) {
    let credential = Credential {
        token: kit::token(1),
        expires_at: kit::at(0) + expires_in,
        hub: kit::origin(),
    };
    TokenStore::open(home)
        .save_new(&credential)
        .expect("stored");
}

pub fn enrolled() -> Rig {
    rig(Some(TimeDelta::days(300)))
}

impl Rig {
    pub fn env(&self) -> impl Fn(&str) -> Option<String> + use<> {
        let home = self.dir.path().display().to_string();
        move |name| match name {
            "HOME" => Some(home.clone()),
            "XDG_RUNTIME_DIR" => Some("/run/user/test".to_owned()),
            _ => None,
        }
    }

    pub fn host(&self) -> Host<'_> {
        Host {
            platform: Platform::Linux,
            uid: self.uid,
            program: &self.program,
            home: &self.home,
            now: kit::at(0),
        }
    }

    pub fn unit(&self) -> PathBuf {
        self.dir
            .path()
            .join(".config/systemd/user/anima-agent.service")
    }

    /// Runs `install` for the user, returning the commands it ran.
    pub fn install(&self, succeed: bool) -> (Result<PathBuf, InstallError>, Vec<String>) {
        let mut ran = Vec::new();
        let result = install(Target::User, &self.host(), self.env(), |step: &Step| {
            ran.push(format!("{} {}", step.program, step.args.join(" ")));
            Ok(succeed)
        });
        (result, ran)
    }
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("a file")
}
