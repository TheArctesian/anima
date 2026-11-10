//! `anima-agent install` for the user against a temporary HOME. The system unit needs root and
//! `/etc`, so only its pure parts are tested (`units::plan`, `units::ready`).

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use anima_agent::config::home::Home;
use anima_agent::config::private_file;
use anima_agent::readers::Platform;
use anima_agent::token_store::{Credential, TokenStore};
use anima_agent::units::install::{Host, InstallError, install};
use anima_agent::units::plan::Target;
use anima_agent::units::ready::NotReady;
use anima_agent::units::step::Step;
use chrono::TimeDelta;
use tempfile::TempDir;

use crate::kit;

struct Rig {
    dir: TempDir,
    program: PathBuf,
    home: Home,
    uid: u32,
}

/// A HOME whose default agent folder holds a token expiring `expires_in` after `kit::at(0)`
/// (none when `None`), and a stand-in binary.
fn rig(expires_in: Option<TimeDelta>) -> Rig {
    let dir = tempfile::tempdir().expect("a folder");
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

fn enrol(home: &Home, expires_in: TimeDelta) {
    let credential = Credential {
        token: kit::token(1),
        expires_at: kit::at(0) + expires_in,
        hub: kit::origin(),
    };
    TokenStore::open(home)
        .save_new(&credential)
        .expect("stored");
}

fn enrolled() -> Rig {
    rig(Some(TimeDelta::days(300)))
}

impl Rig {
    fn env(&self) -> impl Fn(&str) -> Option<String> + use<> {
        let home = self.dir.path().display().to_string();
        move |name| match name {
            "HOME" => Some(home.clone()),
            "XDG_RUNTIME_DIR" => Some("/run/user/test".to_owned()),
            _ => None,
        }
    }

    fn host(&self) -> Host<'_> {
        Host {
            platform: Platform::Linux,
            uid: self.uid,
            program: &self.program,
            home: &self.home,
            now: kit::at(0),
        }
    }

    fn unit(&self) -> PathBuf {
        self.dir
            .path()
            .join(".config/systemd/user/anima-agent.service")
    }

    /// Runs `install` for the user, returning the commands it ran.
    fn install(&self, succeed: bool) -> (Result<PathBuf, InstallError>, Vec<String>) {
        let mut ran = Vec::new();
        let result = install(Target::User, &self.host(), self.env(), |step: &Step| {
            ran.push(format!("{} {}", step.program, step.args.join(" ")));
            Ok(succeed)
        });
        (result, ran)
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("a file")
}

#[test]
fn met_019_install_writes_the_user_unit_with_this_binary_s_path_then_enables_and_restarts_it() {
    let rig = enrolled();
    let (result, ran) = rig.install(true);
    assert_eq!(result.expect("installed"), rig.unit());
    let program = rig.program.canonicalize().expect("path");
    let text = read(&rig.unit());
    assert!(text.contains(&format!("\nExecStart={} run\n", program.display())));
    assert_eq!(text.matches(program.to_str().expect("utf-8")).count(), 1);
    assert_eq!(
        fs::metadata(rig.unit()).expect("meta").permissions().mode() & 0o777,
        0o644
    );
    assert_eq!(
        ran,
        [
            "/usr/bin/systemctl --user daemon-reload",
            "/usr/bin/systemctl --user enable anima-agent.service",
            "/usr/bin/systemctl --user restart anima-agent.service",
        ]
    );
}

#[test]
fn met_019_systemctl_user_is_given_the_session_bus_folder() {
    let rig = enrolled();
    let mut envs = Vec::new();
    install(Target::User, &rig.host(), rig.env(), |step: &Step| {
        envs.push(step.env.clone());
        Ok(true)
    })
    .expect("installed");
    assert_eq!(envs.len(), 3);
    for env in envs {
        assert_eq!(env, [("XDG_RUNTIME_DIR", "/run/user/test".to_owned())]);
    }
}

#[test]
fn met_019_install_run_twice_replaces_the_unit_whole_and_leaves_no_temporary_file() {
    let rig = enrolled();
    rig.install(true).0.expect("first");
    fs::write(rig.unit(), "stale and longer than the unit\n".repeat(500)).expect("stale");
    let first = {
        let (result, _) = rig.install(true);
        result.expect("second");
        read(&rig.unit())
    };
    rig.install(true).0.expect("third");
    assert_eq!(read(&rig.unit()), first);
    assert!(!first.contains("stale"));
    let names: Vec<_> = fs::read_dir(rig.unit().parent().expect("dir"))
        .expect("dir")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(names, ["anima-agent.service"]);
}

#[test]
fn met_019_install_refuses_an_agent_that_is_not_enrolled_and_writes_nothing() {
    let rig = rig(None);
    let (result, ran) = rig.install(true);
    let error = result.expect_err("refused");
    assert!(matches!(
        error,
        InstallError::NotReady(NotReady::NotEnrolled)
    ));
    assert!(error.to_string().contains("anima-agent enrol -"));
    assert!(!rig.unit().exists());
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_an_expired_token_and_writes_nothing() {
    let rig = rig(Some(TimeDelta::seconds(-1)));
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::NotReady(NotReady::Expired)
    ));
    assert!(!rig.unit().exists());
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_an_agent_folder_the_unit_s_agent_would_not_find() {
    let rig = enrolled();
    let elsewhere = Home::at(rig.dir.path().join("elsewhere/anima-agent"));
    enrol(&elsewhere, TimeDelta::days(300));
    let host = Host {
        home: &elsewhere,
        ..rig.host()
    };
    let result = install(Target::User, &host, rig.env(), |_| Ok(true));
    assert!(matches!(
        result,
        Err(InstallError::NotReady(NotReady::OtherHome { .. }))
    ));
    assert!(!rig.unit().exists());
}

#[test]
fn met_019_install_refuses_a_binary_in_a_folder_other_users_can_replace_in() {
    let rig = enrolled();
    let bin = rig.dir.path().join("bin");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o777)).expect("chmod");
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::UntrustedFolder(folder) if folder == bin.canonicalize().expect("path")
    ));
    assert!(!rig.unit().exists());
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_a_binary_other_users_can_write() {
    let rig = enrolled();
    fs::set_permissions(&rig.program, fs::Permissions::from_mode(0o777)).expect("chmod");
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::UntrustedBinary(file) if file == rig.program.canonicalize().expect("path")
    ));
    assert!(!rig.unit().exists());
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_a_unit_folder_that_links_to_one_others_can_write() {
    let rig = enrolled();
    let open = rig.dir.path().join("open");
    fs::create_dir(&open).expect("open");
    fs::set_permissions(&open, fs::Permissions::from_mode(0o777)).expect("chmod");
    fs::create_dir_all(rig.dir.path().join(".config/systemd")).expect("systemd");
    std::os::unix::fs::symlink(&open, rig.dir.path().join(".config/systemd/user")).expect("link");
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::UntrustedFolder(folder) if folder == open.canonicalize().expect("path")
    ));
    assert_eq!(
        fs::read_dir(&open).expect("open").count(),
        0,
        "nothing written"
    );
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_a_binary_whose_path_systemd_would_misread() {
    let rig = enrolled();
    let odd = rig.dir.path().join("bin/anima agent%h");
    fs::rename(&rig.program, &odd).expect("rename");
    let host = Host {
        program: &odd,
        ..rig.host()
    };
    let result = install(Target::User, &host, rig.env(), |_| Ok(true));
    assert!(matches!(result, Err(InstallError::Program(_))));
    assert!(!rig.unit().exists());
}

#[test]
fn met_019_a_failing_command_is_an_error_naming_it_and_a_missing_one_is_too() {
    let rig = enrolled();
    let (result, ran) = rig.install(false);
    let error = result.expect_err("failed");
    assert!(matches!(&error, InstallError::Step { .. }));
    assert!(error.to_string().contains("daemon-reload"));
    assert_eq!(ran.len(), 1, "the later command never ran");
    let missing = install(Target::User, &rig.host(), rig.env(), |_| {
        Err(io::Error::from(io::ErrorKind::NotFound))
    });
    assert!(matches!(missing, Err(InstallError::Run { .. })));
}

#[test]
fn met_019_system_install_without_root_is_refused_before_anything_is_touched() {
    let rig = enrolled();
    let mut ran = false;
    let host = Host {
        uid: 1000,
        ..rig.host()
    };
    let result = install(Target::System, &host, rig.env(), |_| {
        ran = true;
        Ok(true)
    });
    let error = result.expect_err("refused");
    assert!(error.to_string().contains("sudo"));
    assert!(!ran);
}
