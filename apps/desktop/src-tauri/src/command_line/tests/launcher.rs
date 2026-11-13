//! The launcher as `sh` runs it (INS-005, OD-18): a stand-in `AppImage`, a shell script, records
//! its arguments, echoes its input and exits with a chosen code. Nothing real is started.

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::command_line::launcher::{appimage_of, script};

/// Paths a launcher must hold exactly: spaces, quotes, `$`, backquotes, a backslash, a `*`.
const TRICKY: &[&str] = &[
    "/home/ann/Apps/Anima-x86_64.AppImage",
    "/home/ann/My Apps/Anima.AppImage",
    "/home/ann/Ann's apps/Anima.AppImage",
    "/home/ann/'quoted'/x.AppImage",
    "/home/ann/$HOME/`id`/a\\b/*.AppImage",
    "/home/ann/Äpps/Ânima.AppImage",
];

#[test]
fn ins_005_the_launcher_names_its_appimage_and_is_known_again() {
    for path in TRICKY {
        let text = script(Path::new(path)).expect("a launcher");
        assert_eq!(appimage_of(&text), Some(PathBuf::from(path)), "{path}");
    }
}

#[test]
fn ins_005_a_path_the_launcher_cannot_hold_gives_no_launcher() {
    for path in [
        "Anima.AppImage",
        "/home/ann/A\nB.AppImage",
        "/a/\u{7}.AppImage",
    ] {
        assert_eq!(script(Path::new(path)), None, "{path:?}");
    }
}

#[test]
fn ins_006_a_file_without_the_mark_is_not_a_launcher() {
    let text = script(Path::new(TRICKY[0])).unwrap();
    assert_eq!(
        appimage_of(&text.replacen("#!/bin/sh", "#!/bin/bash", 1)),
        None
    );
    assert_eq!(appimage_of(&text.replace("Anima desktop", "other")), None);
    assert_eq!(appimage_of("#!/bin/sh\nexec anima \"$@\"\n"), None);
    assert_eq!(appimage_of(""), None);
}

struct Stage {
    _dir: tempfile::TempDir,
    folder: PathBuf,
}

fn stage() -> Stage {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("with space's $x");
    fs::create_dir(&folder).unwrap();
    Stage { _dir: dir, folder }
}

fn write_mode(path: &Path, text: &str, mode: u32) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// A stand-in `AppImage`: one argument per line on standard output, then its input, exit 3.
const STAND_IN: &str =
    "#!/bin/sh\nfor a in \"$@\"; do printf '[%s]\\n' \"$a\"; done\ncat\nexit 3\n";

fn run(launcher: &Path, args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new("/bin/sh")
        .arg(launcher)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn ins_005_the_launcher_passes_arguments_input_output_and_exit_code_unchanged() {
    let stage = stage();
    let appimage = stage.folder.join("Anima's.AppImage");
    write_mode(&appimage, STAND_IN, 0o755);
    let launcher = stage.folder.join("anima");
    write_mode(&launcher, &script(&appimage).unwrap(), 0o755);
    let output = run(
        &launcher,
        &["device", "two words", "", "$HOME", "it's", "*"],
        "piped in\n",
    );
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "[--anima-cli]\n[device]\n[two words]\n[]\n[$HOME]\n[it's]\n[*]\npiped in\n"
    );
}

#[test]
fn ins_007_the_launcher_runs_the_new_file_after_an_update_at_the_same_path() {
    let stage = stage();
    let appimage = stage.folder.join("Anima.AppImage");
    write_mode(&appimage, STAND_IN, 0o755);
    let launcher = stage.folder.join("anima");
    write_mode(&launcher, &script(&appimage).unwrap(), 0o755);
    // The updater moves the old file away, then writes the new one at the same path.
    fs::rename(&appimage, stage.folder.join("old.AppImage")).unwrap();
    write_mode(&appimage, "#!/bin/sh\necho new\nexit 0\n", 0o755);
    let output = run(&launcher, &[], "");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "new\n");
}

#[test]
fn ins_005_a_moved_or_deleted_appimage_is_told_with_exit_127() {
    let stage = stage();
    let appimage = stage.folder.join("Anima.AppImage");
    let launcher = stage.folder.join("anima");
    write_mode(&launcher, &script(&appimage).unwrap(), 0o755);
    let output = run(&launcher, &["device", "list"], "");
    assert_eq!(output.status.code(), Some(127));
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(said.contains("is not at"), "{said}");
    assert!(
        said.contains(&appimage.to_string_lossy().into_owned()),
        "{said}"
    );
    assert!(said.contains("moved or deleted"), "{said}");
}
