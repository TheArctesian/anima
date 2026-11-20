//! Native supervisor boundaries exercised in Windows, without a Unix shell or WSL.
use crate::{
    child::{Child, Spec, Stop, Stopped},
    log_file::LogFile,
};
use std::{ffi::OsString, path::Path, time::Duration};
use tokio::{io::AsyncWriteExt, sync::mpsc};

const SOON: Duration = Duration::from_secs(20);

fn folder() -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    anima_windows::files::protect(folder.path()).unwrap();
    folder
}

fn start(folder: &Path, script: &str) -> Child {
    let root = std::env::var_os("SystemRoot").unwrap();
    let spec = Spec {
        name: "member",
        program: Path::new(&root).join("System32/WindowsPowerShell/v1.0/powershell.exe"),
        args: [
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ]
        .map(OsString::from)
        .to_vec(),
        env: vec![("ONLY", "this".into())],
    };
    let (ended, _) = mpsc::unbounded_channel();
    Child::start(
        &spec,
        LogFile::open(&folder.join("member.log"), 1 << 20).unwrap(),
        ended,
    )
    .unwrap()
}

#[tokio::test]
async fn a_native_member_logs_both_outputs_and_stops_through_its_private_pipe() {
    let folder = folder();
    let mut child = start(
        folder.path(),
        "[Console]::Out.WriteLine($env:ONLY); [Console]::Error.WriteLine('ready'); if ([Console]::ReadLine() -ne 'stop') { exit 9 }",
    );
    child
        .line_where(SOON, |line| (line == "ready").then_some(()))
        .await
        .unwrap();
    assert!(matches!(child.stop(SOON).await.unwrap(), Stopped::Ended(status) if status.success()));
    let log = std::fs::read_to_string(folder.path().join("member.log")).unwrap();
    assert_eq!(log.replace('\r', ""), "this\nready\n");
    assert!(anima_windows::files::secure(&folder.path().join("member.log"), true).unwrap());
}

#[tokio::test]
async fn a_native_member_ignoring_shutdown_is_killed_at_the_limit() {
    let folder = folder();
    let mut child = start(
        folder.path(),
        "[Console]::Out.WriteLine('ready'); Start-Sleep -Seconds 60",
    );
    child
        .line_where(SOON, |line| (line == "ready").then_some(()))
        .await
        .unwrap();
    assert_eq!(
        child.stop(Duration::from_millis(100)).await.unwrap(),
        Stopped::Killed
    );
}

#[tokio::test]
async fn the_private_control_socket_ignores_invalid_requests_and_keeps_a_stop_request() {
    let folder = folder();
    let path = folder.path().join("stop.sock");
    let (_ask, stop) = Stop::new();
    let mut stop = stop.with_control(&path).unwrap();
    let mut invalid = anima_windows::socket::Stream::connect(&path).await.unwrap();
    invalid.write_all(b"wrong").await.unwrap();
    drop(invalid);
    assert!(!stop.is_asked());
    let mut valid = anima_windows::socket::Stream::connect(&path).await.unwrap();
    valid.write_all(b"stop\n").await.unwrap();
    tokio::time::timeout(SOON, stop.asked()).await.unwrap();
    assert!(stop.is_asked());
}

#[test]
fn native_program_copy_keeps_executables_dlls_and_digest_then_replaces_the_previous_copy() {
    use crate::{Carried, Folders, members::PROGRAMS, programs};
    let folder = folder();
    let source = Carried::installed_in(&folder.path().join("bundle"));
    for directory in [
        source.programs().to_path_buf(),
        source.tree().join("bin"),
        source.tree().join("lib"),
        source.tree().join("share"),
    ] {
        anima_database::owner_only::folder(&directory).unwrap();
    }
    for name in PROGRAMS {
        std::fs::write(source.program(name), b"native fixture").unwrap();
    }
    for name in [
        "postgres.exe",
        "initdb.exe",
        "pg_ctl.exe",
        "pg_isready.exe",
        "libpq.dll",
    ] {
        std::fs::write(source.tree().join("bin").join(name), b"runtime fixture").unwrap();
    }
    let folders = Folders::new(folder.path().join("hub"), folder.path().join("s")).unwrap();
    programs::copy(&source, &folders).unwrap();
    programs::put_in_place(&folders).unwrap();
    let installed = Carried::installed_in(&folders.programs());
    assert_eq!(
        programs::digest::of(&source).unwrap(),
        programs::digest::of(&installed).unwrap()
    );
    assert!(anima_windows::files::secure(&installed.program("anima-hub"), true).unwrap());
    assert!(installed.tree().join("bin/libpq.dll").is_file());
    std::fs::write(source.program("anima-hub"), b"updated fixture").unwrap();
    programs::copy(&source, &folders).unwrap();
    programs::put_in_place(&folders).unwrap();
    assert_eq!(
        std::fs::read(installed.program("anima-hub")).unwrap(),
        b"updated fixture"
    );
    assert!(!folders.programs_replaced().exists());
}
