//! The device link that a unit of the user keeps (HMD-058): the shell's calls of
//! `anima-link unit` and `ctl status`, with a stand-in link, and the watch of a running link.

use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use super::super::{
    copy,
    device::DeviceLink,
    kept::{self, Kept},
    printed,
    state::{Phase, Reason},
};
use super::runs::{stand_in, until};

/// `anima-link` as the shell asks it: `unit install` makes the mark `unit` in the state folder,
/// `unit remove` deletes it, `unit show` and `ctl status` answer from it. The file of the unit
/// is `unit.file` beside it. Each call is noted.
const UNIT_LINK: &str = r#"
here=$(dirname "$0")
echo "$*" >> "$here/calls"
case "$1 $2" in
"unit install") [ "$3 $5" = "-state -listen" ] || exit 9; echo "$6" > "$here/unit"; echo "<string>$4</string>" > "$here/unit.file" ;;
"unit remove") rm -f "$here/unit" "$here/unit.file" ;;
"unit show") if [ -f "$here/unit" ]; then echo "{\"installed\":true,\"file\":\"$here/unit.file\"}"; else echo "{\"installed\":false,\"file\":\"$here/unit.file\"}"; fi ;;
"ctl -state") [ -f "$here/unit" ] || exit 1; echo "{\"role\":\"device\",\"local\":\"$(cat "$here/unit")\",\"path\":\"direct\",\"latencyMs\":1}" ;;
*) exit 9 ;;
esac
"#;

pub fn kept_in(folder: &std::path::Path, port: u16) -> Kept {
    Kept {
        program: stand_in(folder, UNIT_LINK),
        folder: folder.to_path_buf(),
        port,
        daemon_socket: None,
        env: Vec::new(),
    }
}

#[test]
fn hmd_058_the_unit_is_installed_for_the_folder_and_the_port_then_asked_and_removed() {
    let folder = tempfile::tempdir().unwrap();
    let link = kept_in(folder.path(), 41001);

    assert!(!link.keeps_this_folder());
    assert_eq!(link.serving(), None, "no link runs before the unit");
    link.install().unwrap();
    assert!(link.keeps_this_folder());
    assert_eq!(link.serving(), Some(41001));
    link.remove().unwrap();
    assert!(!link.keeps_this_folder());

    let state = folder.path().display();
    let calls = fs::read_to_string(folder.path().join("calls")).unwrap();
    assert_eq!(
        calls.lines().collect::<Vec<_>>(),
        [
            "unit show".to_owned(),
            format!("ctl -state {state} status"),
            format!("unit install -state {state} -listen 127.0.0.1:41001"),
            "unit show".to_owned(),
            format!("ctl -state {state} status"),
            "unit remove".to_owned(),
            "unit show".to_owned(),
        ]
    );
}

#[test]
fn hmd_058_a_unit_that_runs_a_different_folder_is_not_the_unit_of_this_one() {
    let folder = tempfile::tempdir().unwrap();
    let link = kept_in(folder.path(), 41001);
    assert!(!link.keeps_this_folder(), "no unit yet");
    link.install().unwrap();
    assert!(link.keeps_this_folder());

    // The same unit file and program, asked for the folder of a different copy of the app.
    let other = Kept {
        folder: folder.path().join("other"),
        ..link.clone()
    };
    assert!(folder.path().join("unit.file").is_file());
    assert!(!other.keeps_this_folder());
    let longer = Kept {
        folder: folder.path().with_extension("old"),
        ..link
    };
    assert!(
        !longer.keeps_this_folder(),
        "a longer path is a different one"
    );
}

#[test]
fn the_folder_of_a_unit_is_one_whole_argument_of_its_file() {
    let agent = "<array>\n<string>/a/device</string>\n</array>";
    assert!(printed::unit_runs_folder(agent, "/a/device"));
    assert!(!printed::unit_runs_folder(agent, "/a"));
    assert!(!printed::unit_runs_folder(agent, "/a/dev"));
    let systemd =
        r#"ExecStart="/b/anima-link" "device" "-state" "/a/device" "-listen" "127.0.0.1:1""#;
    assert!(printed::unit_runs_folder(systemd, "/a/device"));
    assert!(!printed::unit_runs_folder(systemd, "/a/device.old"));
    let task = r"<Arguments>&#34;device&#34; &#34;-state&#34; &#34;C:\Users\Ada &amp; Bob\device&#34;</Arguments>";
    assert!(printed::unit_runs_folder(
        task,
        r"C:\Users\Ada & Bob\device"
    ));
    assert!(!printed::unit_runs_folder(task, r"C:\Users\Ada & Bob"));
    assert!(!printed::unit_runs_folder(
        task,
        r"C:\Users\Ada & Bob\device.old"
    ));
    assert_eq!(
        printed::unit_file(br#"{"installed":false,"file":"/x"}"#),
        None
    );
    assert_eq!(
        printed::unit_file(br#"{"installed":true,"file":"x"}"#),
        None
    );
    assert_eq!(
        printed::unit_file(br#"{"installed":true,"file":"/x"}"#),
        Some("/x".into())
    );
}

#[test]
fn hmd_058_a_link_program_that_refuses_the_unit_gives_an_error_and_no_unit() {
    let folder = tempfile::tempdir().unwrap();
    let mut link = kept_in(folder.path(), 41001);
    link.program = stand_in(folder.path(), "echo 'unit: refused' >&2; exit 1");
    assert!(link.install().is_err());
    assert!(!link.keeps_this_folder());
    assert_eq!(link.serving(), None);
}

#[test]
fn hmd_058_the_app_watches_a_kept_link_and_its_stop_does_not_end_the_link() {
    let folder = tempfile::tempdir().unwrap();
    let link = kept_in(folder.path(), 41001);
    link.install().unwrap();
    let device = DeviceLink::default();

    link.clone().watch(&device);
    let up = until(&device, |state| state.phase == Phase::Up);
    assert!(up.serves(41001), "{up:?}");

    device.stop();
    assert_eq!(device.state().phase, Phase::Stopped);
    assert!(
        link.keeps_this_folder(),
        "the unit stays when the app stops"
    );
    assert_eq!(link.serving(), Some(41001));
}

#[test]
fn hmd_058_a_kept_link_that_goes_away_is_down_and_is_up_again_when_it_answers() {
    let answers = Arc::new(AtomicBool::new(false));
    let asked = Arc::clone(&answers);
    let device = DeviceLink::default();

    device.watch(41001, move || {
        asked.load(Ordering::Acquire).then_some(41001)
    });
    let waiting = until(&device, |state| state.phase == Phase::Starting);
    assert_eq!(waiting.phase, Phase::Starting, "no answer yet: not up");

    answers.store(true, Ordering::Release);
    assert!(until(&device, |state| state.phase == Phase::Up).serves(41001));
    answers.store(false, Ordering::Release);
    let down = until(&device, |state| state.phase == Phase::Down);
    assert_eq!(down.reason, Some(Reason::Ended));
    answers.store(true, Ordering::Release);
    assert!(until(&device, |state| state.phase == Phase::Up).serves(41001));
    device.stop();
}

#[test]
fn the_answers_of_the_link_program_are_read_strictly() {
    assert_eq!(
        printed::link_local(br#"{"role":"device","local":"127.0.0.1:41001"}"#),
        Some(41001)
    );
    for refused in [
        &br#"{"role":"hub","local":"127.0.0.1:41001"}"#[..],
        br#"{"role":"device","local":"192.0.2.1:41001"}"#,
        br#"{"role":"device","local":"127.0.0.1:0"}"#,
        br#"{"role":"device"}"#,
        b"not json",
    ] {
        assert_eq!(printed::link_local(refused), None);
    }
    assert!(printed::unit_installed(
        br#"{"installed":true,"file":"/x"}"#
    ));
    for not in [
        &br#"{"installed":false}"#[..],
        br#"{"installed":"yes"}"#,
        b"",
    ] {
        assert!(!printed::unit_installed(not));
    }
}

#[test]
fn the_unit_commands_get_the_home_and_the_session_of_the_user_and_nothing_else() {
    let env = kept::environment(|name| match name {
        "HOME" => Some("/home/me".into()),
        "XDG_RUNTIME_DIR" => Some("/run/user/1000".into()),
        "XDG_CONFIG_HOME" => Some(String::new().into()),
        "SECRET" => Some("no".into()),
        _ => None,
    });
    assert_eq!(
        env,
        [
            ("HOME", "/home/me".into()),
            ("XDG_RUNTIME_DIR", "/run/user/1000".into())
        ]
    );
}

#[test]
fn the_copy_of_the_link_is_in_the_folder_of_the_app_and_a_unit_of_an_earlier_app_is_found() {
    let root = tempfile::tempdir().unwrap();
    let (application, home) = (root.path().join("app"), root.path().join("home"));
    let new_copy = application.join("link/program/anima-link");
    let earlier = home.join(".local/bin/anima-link");
    assert_eq!(copy::destination(&application), new_copy);
    assert_eq!(copy::earlier_copy(&home), earlier);
    assert_eq!(
        copy::current_copy(&application, &home),
        new_copy,
        "no copy yet"
    );

    fs::create_dir_all(earlier.parent().unwrap()).unwrap();
    fs::write(&earlier, b"link").unwrap();
    assert_eq!(
        copy::current_copy(&application, &home),
        earlier,
        "a unit of an earlier app runs ~/.local/bin/anima-link"
    );

    fs::create_dir_all(new_copy.parent().unwrap()).unwrap();
    fs::write(&new_copy, b"link").unwrap();
    assert_eq!(copy::current_copy(&application, &home), new_copy);
}

#[test]
fn hmd_058_the_folder_of_a_hosted_link_is_marked_once_and_a_link_is_no_mark() {
    let folder = tempfile::tempdir().unwrap();
    assert!(!super::super::folder::wants_unit(folder.path()));
    super::super::folder::want_unit(folder.path()).unwrap();
    super::super::folder::want_unit(folder.path()).unwrap();
    assert!(super::super::folder::wants_unit(folder.path()));

    let other = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/etc/hosts", other.path().join("anima-unit")).unwrap();
    assert!(!super::super::folder::wants_unit(other.path()));
}

#[test]
fn hmd_054_the_unit_is_given_the_socket_of_the_daemon_when_its_folder_is_private() {
    let folder = tempfile::tempdir().unwrap();
    let daemon = folder.path().join("made/anima-daemon");
    let socket = kept::private_socket(&daemon).expect("a folder that the app made");
    assert_eq!(socket, daemon.join("link.sock"));
    let mode = std::os::unix::fs::MetadataExt::mode(&fs::metadata(&daemon).unwrap());
    assert_eq!(mode & 0o777, 0o700);

    let mut link = kept_in(folder.path(), 41001);
    link.daemon_socket = Some(socket.clone());
    link.install().unwrap();
    let calls = fs::read_to_string(folder.path().join("calls")).unwrap();
    assert_eq!(
        calls.lines().last().unwrap(),
        format!(
            "unit install -state {} -listen 127.0.0.1:41001 -daemon-socket {}",
            folder.path().display(),
            socket.display()
        )
    );
}

#[test]
fn the_folder_of_the_daemon_is_where_the_daemon_looks_for_it() {
    use std::path::Path;
    let home = Path::new("/home/me");
    assert_eq!(
        kept::daemon_folder(Path::new("/Users/me"), true, Some(Path::new("/x"))),
        Path::new("/Users/me/Library/Application Support/anima-daemon")
    );
    assert_eq!(
        kept::daemon_folder(home, false, None),
        Path::new("/home/me/.config/anima-daemon")
    );
    assert_eq!(
        kept::daemon_folder(home, false, Some(Path::new("/home/me/conf"))),
        Path::new("/home/me/conf/anima-daemon")
    );
    assert_eq!(
        kept::daemon_folder(home, false, Some(Path::new("relative"))),
        Path::new("/home/me/.config/anima-daemon")
    );
}
