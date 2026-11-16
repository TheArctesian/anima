use std::{
    fs,
    net::{Ipv4Addr, TcpListener},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use super::super::{
    LinkError,
    device::{DeviceLink, Launch},
    folder::{self, LinkFolders},
    joining,
    state::{DeviceState, Phase, Reason},
};

const INVITE_LINE: &str = "anima-invite.1.q83vEjRWeJCrze8SNFZ4kA.hub\n";
const PATIENCE: Duration = Duration::from_secs(10);

fn stand_in(folder: &Path, body: &str) -> PathBuf {
    let program = folder.join("anima-link");
    fs::write(&program, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program
}

fn free_port() -> u16 {
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

fn until(device: &DeviceLink, wanted: impl Fn(&DeviceState) -> bool) -> DeviceState {
    let end = Instant::now() + PATIENCE;
    loop {
        let state = device.state();
        if wanted(&state) || Instant::now() > end {
            return state;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// `anima-link device` as the shell starts it: `device -state DIR -listen ADDR -stop-with-stdin`.
const READY_THEN_WAIT: &str = r#"
[ "$1 $2 $4 $6" = "device -state -listen -stop-with-stdin" ] || exit 9
echo $$ > "$3/pid"
echo "level=INFO msg=\"the device link is ready\" local=$5 key=1a2b3c4d" >&2
cat > /dev/null
"#;

#[test]
fn the_device_link_comes_up_on_its_port_and_ends_when_stopped() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), READY_THEN_WAIT);
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
    device.stop();
    assert_eq!(device.state().phase, Phase::Stopped);
    // The child read the end of its standard input and is gone.
    let gone = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_err();
    assert!(gone);
}

#[test]
fn a_taken_port_is_reported_and_the_link_is_not_started_on_another() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), READY_THEN_WAIT);
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

/// `anima-link join -state DIR` with the invite on standard input.
const JOIN: &str = r#"
[ "$1 $2" = "join -state" ] || exit 9
read -r invite
mkdir -p "$3"
case "$invite" in
  anima-invite.1.q83vEjRWeJCrze8SNFZ4kA.hub)
    echo "This device has joined the hub. Its key fingerprint is 1a2b3c4d."
    echo "Start the link with: anima-link device -state $3" ;;
  *)
    echo "anima-link: devicelink: the hub did not answer; the invite is used, expired or wrong, or the hub is off" >&2
    exit 1 ;;
esac
"#;

#[test]
fn a_join_makes_a_new_folder_with_a_port_that_replaces_the_old_one() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), JOIN);
    let folders = LinkFolders::below(&folder.path().join("app"));
    fs::create_dir_all(&folders.device).unwrap();
    fs::write(folders.device.join("old-key"), "").unwrap();

    let joined = joining::join(&program, &folders, INVITE_LINE, &[]).unwrap();

    assert_eq!(joined.fingerprint, "1a2b3c4d");
    assert_eq!(folder::read_port(&folders.next), Some(joined.port));
    let mode = fs::metadata(folders.next.join("anima-port"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let link_mode = fs::metadata(folders.device.parent().unwrap())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(link_mode & 0o077, 0, "{link_mode:o}");

    folders.take_next().unwrap();
    assert_eq!(folder::read_port(&folders.device), Some(joined.port));
    assert!(!folders.device.join("old-key").exists());
    assert!(!folders.next.exists());
}

#[test]
fn a_join_the_hub_does_not_answer_leaves_the_old_link_alone() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), JOIN);
    let folders = LinkFolders::below(&folder.path().join("app"));
    fs::create_dir_all(&folders.device).unwrap();
    fs::write(folders.device.join("old-key"), "").unwrap();

    let error = joining::join(&program, &folders, "anima-invite.1.used.hub\n", &[]).unwrap_err();

    assert_eq!(error, LinkError::NoAnswer);
    assert!(!folders.next.exists());
    assert!(folders.device.join("old-key").exists());
}

#[test]
fn leaving_removes_every_folder_of_the_link() {
    let folder = tempfile::tempdir().unwrap();
    let folders = LinkFolders::below(folder.path());
    fs::create_dir_all(folders.device.join("inner")).unwrap();
    fs::create_dir_all(&folders.next).unwrap();

    folders.remove().unwrap();

    assert!(!folders.device.exists());
    assert!(!folders.next.exists());
    folders.remove().unwrap();
}
