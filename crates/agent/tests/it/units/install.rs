//! `anima-agent install` for the user against a temporary HOME: what it writes and runs, and
//! when it refuses for the token. The system unit needs root and `/etc`, so only its pure parts
//! are tested (`units::plan`, `units::ready`).

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt as _;

use anima_agent::config::home::Home;
use anima_agent::units::install::{Host, InstallError, install};
use anima_agent::units::plan::Target;
use anima_agent::units::ready::NotReady;
use anima_agent::units::step::Step;
use chrono::TimeDelta;

use super::rig::{enrol, enrolled, read, rig};

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
