use std::{ffi::OsString, fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use super::{
    DevFolders, programs_folder,
    run::{ExchangeError, Invocation, exchange},
};

const SOON: Duration = Duration::from_secs(20);

fn script(folder: &Path, body: &str) -> std::path::PathBuf {
    let program = folder.join("stand-in");
    fs::write(&program, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program
}

#[test]
fn a_release_build_ignores_the_dev_folders_and_a_debug_build_uses_only_them() {
    let executable = Path::new("/Applications/Anima.app/Contents/MacOS/anima-desktop");
    let dev = DevFolders {
        programs: Some("/Users/ada/anima/target/debug".into()),
        tree: Some("/Users/ada/pg".into()),
    };

    assert_eq!(
        programs_folder(executable, true, &dev).as_deref(),
        Some(Path::new("/Applications/Anima.app/Contents/MacOS"))
    );
    assert_eq!(
        programs_folder(executable, false, &dev).as_deref(),
        Some(Path::new("/Users/ada/anima/target/debug"))
    );
    assert_eq!(
        programs_folder(executable, false, &DevFolders::default()),
        None
    );
    let relative = DevFolders {
        programs: Some("target/debug".into()),
        tree: None,
    };
    assert_eq!(programs_folder(executable, false, &relative), None);
}

#[test]
fn an_exchange_gives_the_input_on_standard_input_only() {
    let folder = tempfile::tempdir().unwrap();
    let program = script(
        folder.path(),
        r#"read -r first; read -r second; echo "$first+$second $#""#,
    );
    let invocation = Invocation {
        program: &program,
        args: &[],
        env: &[],
        limit: SOON,
    };

    let printed = exchange(&invocation, Some(b"one\ntwo\n")).unwrap();

    assert_eq!(&printed[..], b"one+two 0\n");
}

#[test]
fn a_failed_exchange_gives_its_exit_code_and_keeps_what_it_said_out_of_text() {
    let folder = tempfile::tempdir().unwrap();
    let program = script(folder.path(), "echo 'secret-ABC' >&2; exit 3");
    let create = OsString::from("create");
    let invocation = Invocation {
        program: &program,
        args: &[&create],
        env: &[],
        limit: SOON,
    };

    let error = exchange(&invocation, None).unwrap_err();

    let ExchangeError::Exited { code, said } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(*code, Some(3));
    assert!(said.text().contains("secret-ABC"));
    assert!(
        !format!("{error} {error:?}").contains("secret"),
        "{error:?}"
    );
}

#[test]
fn an_exchange_that_does_not_end_is_killed_at_the_limit() {
    let folder = tempfile::tempdir().unwrap();
    let program = script(folder.path(), "exec sleep 30");
    let invocation = Invocation {
        program: &program,
        args: &[],
        env: &[],
        limit: Duration::from_millis(200),
    };

    assert_eq!(
        exchange(&invocation, Some(b"x\n")).unwrap_err(),
        ExchangeError::TimedOut
    );
}
