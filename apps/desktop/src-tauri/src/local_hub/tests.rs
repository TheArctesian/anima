use std::{
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    Changing, LocalHubError, carried, environment, install_args, parse_status,
    program::{self, ProgramError},
};

const SOON: Duration = Duration::from_secs(20);

/// A bundle as the builder makes it, with a stand-in for the program.
fn bundle(folder: &Path, macos: bool, script: &str) -> PathBuf {
    let (programs, tree) = if macos {
        (
            "Anima.app/Contents/MacOS",
            "Anima.app/Contents/Resources/database",
        )
    } else {
        ("usr/bin", "usr/lib/Anima/database")
    };
    fs::create_dir_all(folder.join(programs)).unwrap();
    fs::create_dir_all(folder.join(tree).join("bin")).unwrap();
    fs::write(folder.join(tree).join("bin/postgres"), "").unwrap();
    let program = folder.join(programs).join("anima-local-hub");
    fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    folder.join(programs).join("anima-desktop")
}

fn env_of(pairs: &[(&'static str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let pairs: Vec<(&str, OsString)> = pairs
        .iter()
        .map(|(name, value)| (*name, value.into()))
        .collect();
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.clone())
    }
}

#[test]
fn a_release_build_finds_the_program_and_the_tree_where_the_builder_puts_them() {
    for macos in [true, false] {
        let folder = tempfile::tempdir().unwrap();
        let executable = bundle(folder.path(), macos, "true");

        let found = carried::beside(&executable, macos, true).unwrap();

        assert_eq!(found.program, executable.with_file_name("anima-local-hub"));
        assert!(
            found.tree.join("bin/postgres").is_file(),
            "{}",
            found.tree.display()
        );
        assert!(found.tree.starts_with(folder.path()));
    }
}

#[test]
fn a_debug_build_and_a_bundle_without_the_tree_carry_no_hub() {
    let folder = tempfile::tempdir().unwrap();
    let executable = bundle(folder.path(), true, "true");

    assert_eq!(
        carried::beside(&executable, true, false),
        Err(LocalHubError::NotCarried)
    );
    // The Linux place in a macOS bundle: no tree there.
    assert_eq!(
        carried::beside(&executable, false, true),
        Err(LocalHubError::NotCarried)
    );
    fs::remove_file(executable.with_file_name("anima-local-hub")).unwrap();
    assert_eq!(
        carried::beside(&executable, true, true),
        Err(LocalHubError::NotCarried)
    );
}

#[test]
fn the_program_sees_only_the_users_folders_and_the_session_bus() {
    let read = env_of(&[
        ("HOME", "/home/ada"),
        ("XDG_RUNTIME_DIR", "/run/user/1000"),
        ("LD_PRELOAD", "/tmp/x.so"),
        ("PATH", "/tmp"),
    ]);

    let env = environment(read).unwrap();

    let names: Vec<&str> = env.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["HOME", "XDG_RUNTIME_DIR"]);
}

#[test]
fn without_an_absolute_home_nothing_runs() {
    assert_eq!(environment(env_of(&[])), Err(LocalHubError::NoHome));
    assert_eq!(
        environment(env_of(&[("HOME", "relative")])),
        Err(LocalHubError::NoHome)
    );
}

#[test]
fn install_names_the_tree_of_the_bundle_and_nothing_from_the_web_view() {
    let args = install_args(Path::new(
        "/Applications/Anima.app/Contents/Resources/database",
    ));

    assert_eq!(
        args,
        [
            "install",
            "--tree",
            "/Applications/Anima.app/Contents/Resources/database"
        ]
    );
}

#[test]
fn the_program_runs_with_the_given_environment_and_its_answer_is_returned() {
    let folder = tempfile::tempdir().unwrap();
    let executable = bundle(
        folder.path(),
        true,
        r#"echo "$1 home=$HOME path=${PATH-unset} cwd=$(pwd)""#,
    );
    let found = carried::beside(&executable, true, true).unwrap();
    let env = [("HOME", OsString::from("/home/ada"))];

    let printed = program::run(&found.program, &[&OsString::from("status")], &env, SOON).unwrap();

    let printed = String::from_utf8(printed).unwrap();
    assert!(printed.starts_with("status home=/home/ada "), "{printed}");
    assert!(printed.trim_end().ends_with("cwd=/"), "{printed}");
    // `/bin/sh` gives itself a PATH when it has none; the app's own PATH is not passed on.
    assert!(!printed.contains(env!("CARGO_MANIFEST_DIR")), "{printed}");
}

#[test]
fn a_program_that_fails_gives_its_first_error_line() {
    let folder = tempfile::tempdir().unwrap();
    let script =
        "echo 'anima-local-hub: the hub of this folder still runs' >&2; echo more >&2; exit 1";
    let executable = bundle(folder.path(), true, script);
    let found = carried::beside(&executable, true, true).unwrap();

    let error = program::run(&found.program, &[], &[], SOON).unwrap_err();

    assert_eq!(
        error,
        ProgramError::Failed("anima-local-hub: the hub of this folder still runs".to_owned())
    );
    assert_eq!(LocalHubError::from(&error), LocalHubError::Failed);
}

#[test]
fn a_program_that_does_not_end_is_killed_at_the_limit() {
    let folder = tempfile::tempdir().unwrap();
    let executable = bundle(folder.path(), true, "exec sleep 30");
    let found = carried::beside(&executable, true, true).unwrap();

    let error = program::run(&found.program, &[], &[], Duration::from_millis(200)).unwrap_err();

    assert_eq!(error, ProgramError::TimedOut);
    assert_eq!(LocalHubError::from(&error), LocalHubError::TimedOut);
}

#[test]
fn only_a_status_object_is_passed_to_the_client() {
    let ready = br#"{"installed":true,"running":true,"state":"ready","hub":"127.0.0.1:8443","recovered":false}"#;

    assert_eq!(parse_status(ready).unwrap()["hub"], "127.0.0.1:8443");
    for not_a_status in [
        &b"ready"[..],
        b"{}",
        br#"{"installed":"yes","running":true}"#,
        b"[true,true]",
    ] {
        assert_eq!(parse_status(not_a_status), Err(LocalHubError::Failed));
    }
}

#[test]
fn one_change_at_a_time() {
    let changing = Changing::default();

    let first = changing.claim().unwrap();
    assert_eq!(changing.claim().err(), Some(LocalHubError::Busy));
    drop(first);

    assert!(changing.claim().is_ok());
}

#[test]
fn each_error_has_its_own_code_and_fixed_text() {
    let mut codes: Vec<&str> = LocalHubError::ALL
        .iter()
        .map(|error| error.code())
        .collect();
    codes.sort_unstable();
    codes.dedup();

    assert_eq!(codes.len(), LocalHubError::ALL.len());
    let sent = serde_json::to_value(LocalHubError::Busy).unwrap();
    assert_eq!(sent["code"], "busy");
    assert_eq!(sent["message"], "an install or a remove is already running");
}
