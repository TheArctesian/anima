//! The device link child, run as a stand-in script.

use std::{
    fs,
    net::{Ipv4Addr, TcpListener},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use super::super::{
    device::{DeviceLink, Launch, STOP_PATIENCE},
    printed,
    state::{DeviceState, Phase, Reason},
};

pub const PATIENCE: Duration = Duration::from_secs(10);

pub fn stand_in(folder: &Path, body: &str) -> PathBuf {
    let program = folder.join("anima-link");
    fs::write(&program, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program
}

pub fn free_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn launch(program: PathBuf, folder: &Path, port: u16) -> Launch {
    Launch {
        program,
        folder: folder.to_path_buf(),
        port,
        env: Vec::new(),
        restart_after: Duration::from_millis(100),
    }
}

pub fn until(device: &DeviceLink, wanted: impl Fn(&DeviceState) -> bool) -> DeviceState {
    let end = Instant::now() + PATIENCE;
    loop {
        let state = device.state();
        if wanted(&state) || Instant::now() > end {
            return state;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// `anima-link device` as the shell starts it: `device -state DIR -listen ADDR
/// -stop-with-stdin`. It logs the ready line, waits for the end of its standard input, then
/// writes `closed` in its state folder and ends, as `-stop-with-stdin` does.
pub fn ready_then_wait() -> String {
    format!(
        r#"
[ "$1 $2 $4 $6" = "device -state -listen -stop-with-stdin" ] || exit 9
echo $$ > "$3/pid"
echo "time=2026-11-15T10:00:00Z level=INFO {} local=$5 key=1a2b3c4d" >&2
cat > /dev/null
echo closed > "$3/closed"
"#,
        // Inside the shell's double quotes.
        printed::READY.replace('"', "\\\"")
    )
}

#[test]
fn the_device_link_comes_up_on_its_port_and_ends_gracefully_when_stopped() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), &ready_then_wait());
    let port = free_port();
    let device = DeviceLink::default();

    device.start(launch(program, folder.path(), port));
    let up = until(&device, |state| state.phase == Phase::Up);

    assert!(up.serves(port), "{up:?}");
    let pid: i32 = fs::read_to_string(folder.path().join("pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let stopping = Instant::now();
    device.stop();
    assert!(
        stopping.elapsed() < STOP_PATIENCE,
        "the child was killed, not stopped"
    );
    assert_eq!(device.state().phase, Phase::Stopped);
    // The child saw the end of its standard input, ended by itself and is gone.
    assert!(folder.path().join("closed").exists());
    let gone = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_err();
    assert!(gone);
}

#[test]
fn a_taken_port_is_reported_and_the_link_is_not_started_on_another() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), &ready_then_wait());
    let holder = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = holder.local_addr().unwrap().port();
    let device = DeviceLink::default();

    device.start(launch(program, folder.path(), port));
    let down = until(&device, |state| state.reason == Some(Reason::PortTaken));
    device.stop();

    assert_eq!(down.phase, Phase::Down);
    assert_eq!(down.port, Some(port));
    assert!(!folder.path().join("pid").exists());
}

#[test]
fn a_link_that_ends_is_started_again_after_the_pause() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), r#"echo start >> "$3/starts"; exit 1"#);
    let device = DeviceLink::default();
    let starts = folder.path().join("starts");

    device.start(launch(program, folder.path(), free_port()));
    let end = Instant::now() + PATIENCE;
    while fs::read_to_string(&starts).map_or(0, |text| text.lines().count()) < 2
        && Instant::now() < end
    {
        thread::sleep(Duration::from_millis(20));
    }
    device.stop();

    assert!(fs::read_to_string(&starts).unwrap().lines().count() >= 2);
}
