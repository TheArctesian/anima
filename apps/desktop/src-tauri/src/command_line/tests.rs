//! The carrier, folder, launcher, dispatch and state rules are pure; `tests/launcher.rs` runs a
//! written launcher with `/bin/sh` against a stand-in `AppImage`; `tests/flow.rs` installs and
//! removes in a temporary HOME, so no real `PATH` folder is touched.

mod flow;
mod launcher;

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use super::{
    CommandLineError,
    carrier::{self, Carrier},
    directory::{PROFILE_LINE, Proposal, candidates, propose},
    dispatch::{cli_args, typed_in},
    error::Occupant,
    found::Found,
    login_path::path_line,
    state::{Fit, Status, fit, status},
};

const APP_EXE: &str = "/Applications/Anima.app/Contents/MacOS/anima-desktop";
const APP_PROGRAM: &str = "/Applications/Anima.app/Contents/MacOS/anima";

fn bundle() -> Carrier {
    Carrier::Bundle {
        program: PathBuf::from(APP_PROGRAM),
    }
}

#[test]
fn ins_004_the_link_points_at_anima_beside_the_app_executable() {
    assert_eq!(carrier::bundle(Path::new(APP_EXE), true), Ok(bundle()));
    let other = "/Users/ann/Applications/Anima Beta.app/Contents/MacOS/anima-desktop";
    assert!(carrier::bundle(Path::new(other), true).is_ok());
}

#[test]
fn ins_004_a_disk_image_or_a_translocated_app_is_refused() {
    let dmg = "/Volumes/Anima/Anima.app/Contents/MacOS/anima-desktop";
    assert_eq!(
        carrier::bundle(Path::new(dmg), true),
        Err(CommandLineError::DiskImage)
    );
    let moved =
        "/private/var/folders/x/T/AppTranslocation/1F2E/d/Anima.app/Contents/MacOS/anima-desktop";
    assert_eq!(
        carrier::bundle(Path::new(moved), true),
        Err(CommandLineError::Translocated)
    );
}

#[test]
fn ins_002_an_app_without_anima_or_outside_a_bundle_carries_none() {
    assert_eq!(
        carrier::bundle(Path::new(APP_EXE), false),
        Err(CommandLineError::NotBundled)
    );
    let loose = "/Users/ann/anima/target/release/anima-desktop";
    assert_eq!(
        carrier::bundle(Path::new(loose), true),
        Err(CommandLineError::NotBundled)
    );
}

#[test]
fn ins_005_the_launcher_runs_the_appimage_file_not_its_mount() {
    let file = OsStr::new("/home/ann/Apps/Anima-x86_64.AppImage");
    let mount = OsStr::new("/tmp/.mount_AnimaXy12");
    let exe = Path::new("/tmp/.mount_AnimaXy12/usr/bin/anima-desktop");
    assert_eq!(
        carrier::app_image(Some(file), Some(mount), exe, true),
        Ok(Carrier::AppImage {
            file: PathBuf::from(file)
        })
    );
    assert_eq!(
        carrier::app_image(Some(file), Some(mount), exe, false),
        Err(CommandLineError::NotBundled)
    );
}

#[test]
fn ins_005_outside_an_appimage_there_is_no_launcher() {
    let file = OsStr::new("/home/ann/Apps/Anima-x86_64.AppImage");
    let mount = OsStr::new("/tmp/.mount_AnimaXy12");
    let exe = Path::new("/tmp/.mount_AnimaXy12/usr/bin/anima-desktop");
    let elsewhere = Path::new("/usr/bin/anima-desktop");
    for (appimage, appdir, exe) in [
        (None, Some(mount), exe),
        (Some(file), None, exe),
        (Some(file), Some(mount), elsewhere),
        (Some(OsStr::new("Anima.AppImage")), Some(mount), exe),
    ] {
        assert_eq!(
            carrier::app_image(appimage, appdir, exe, true),
            Err(CommandLineError::NotAppImage)
        );
    }
    let newline = OsStr::new("/home/ann/A\nnima.AppImage");
    assert_eq!(
        carrier::app_image(Some(newline), Some(mount), exe, true),
        Err(CommandLineError::Unquotable)
    );
}

#[test]
fn ins_006_only_an_anima_program_in_an_app_bundle_is_an_anima_link() {
    assert!(carrier::is_bundle_program(Path::new(APP_PROGRAM)));
    for other in [
        "/Applications/Anima.app/Contents/MacOS/anima-desktop",
        "/Applications/Anima/Contents/MacOS/anima",
        "Anima.app/Contents/MacOS/anima",
        "/opt/homebrew/bin/anima",
    ] {
        assert!(!carrier::is_bundle_program(Path::new(other)), "{other}");
    }
}

fn home() -> PathBuf {
    PathBuf::from("/Users/ann")
}

#[test]
fn ins_009_the_first_writable_candidate_on_the_path_is_proposed() {
    let [local, bin, usr] = candidates(&home());
    let path = vec![usr.clone(), bin.clone()];
    let all = |_: &Path| true;
    assert_eq!(
        propose(&home(), &path, all),
        Proposal {
            folder: bin,
            on_path: true,
            create: false
        }
    );
    let only_usr = |folder: &Path| folder == usr;
    assert_eq!(propose(&home(), &path, only_usr).folder, usr);
    let none = |_: &Path| false;
    assert_eq!(propose(&home(), &path, none).folder, local);
}

#[test]
fn ins_009_with_no_folder_on_the_path_local_bin_is_made_and_the_profile_line_shown() {
    let [local, ..] = candidates(&home());
    let proposal = propose(&home(), &[PathBuf::from("/usr/bin")], |_| false);
    assert_eq!(
        proposal,
        Proposal {
            folder: local.clone(),
            on_path: false,
            create: true
        }
    );
    let Status::NotInstalled {
        profile_line, path, ..
    } = status(&[], &proposal)
    else {
        panic!("not installed");
    };
    assert_eq!(profile_line, Some(PROFILE_LINE));
    assert_eq!(PathBuf::from(path), local.join("anima"));
}

#[test]
fn ins_009_a_trailing_slash_on_the_path_is_the_same_folder() {
    let path = vec![PathBuf::from("/Users/ann/.local/bin/")];
    assert!(propose(&home(), &path, |_| true).on_path);
}

#[test]
fn ins_006_a_link_or_launcher_is_this_apps_another_anima_apps_or_foreign() {
    let program = PathBuf::from(APP_PROGRAM);
    assert_eq!(fit(Found::Nothing, &bundle()), Fit::Absent);
    assert_eq!(fit(Found::Link(program), &bundle()), Fit::Current);
    let old = PathBuf::from("/Users/ann/Downloads/Anima.app/Contents/MacOS/anima");
    assert_eq!(
        fit(Found::Link(old.clone()), &bundle()),
        Fit::Elsewhere(old)
    );
    let brew = PathBuf::from("/opt/homebrew/Cellar/anima/1/bin/anima");
    assert_eq!(
        fit(Found::Link(brew.clone()), &bundle()),
        Fit::Foreign(Occupant::Link(brew))
    );
    assert_eq!(
        fit(Found::Other(Occupant::File), &bundle()),
        Fit::Foreign(Occupant::File)
    );
    let file = PathBuf::from("/home/ann/Anima.AppImage");
    let appimage = Carrier::AppImage { file: file.clone() };
    assert_eq!(fit(Found::Launcher(file), &appimage), Fit::Current);
    let moved = PathBuf::from("/home/ann/old/Anima.AppImage");
    assert_eq!(
        fit(Found::Launcher(moved.clone()), &appimage),
        Fit::Elsewhere(moved)
    );
}

#[test]
fn ins_006_installed_wins_over_another_apps_entry_which_wins_over_not_installed() {
    let proposal = propose(&home(), &[], |_| false);
    let [local, bin, _] = candidates(&home()).map(|folder| folder.join("anima"));
    let old = PathBuf::from("/Volumes/x/Anima.app/Contents/MacOS/anima");
    let fits = vec![
        (local.clone(), Fit::Elsewhere(old.clone())),
        (bin.clone(), Fit::Current),
    ];
    assert_eq!(
        status(&fits, &proposal),
        Status::Installed {
            path: bin.to_string_lossy().into_owned()
        }
    );
    let fits = vec![(local.clone(), Fit::Elsewhere(old.clone()))];
    assert_eq!(
        status(&fits, &proposal),
        Status::Elsewhere {
            folder: "/Users/ann/.local/bin".into(),
            path: local.to_string_lossy().into_owned(),
            points_at: old.to_string_lossy().into_owned(),
        }
    );
}

#[test]
fn ins_006_the_state_crosses_to_the_client_in_camel_case() {
    let proposal = propose(&home(), &[home().join(".local/bin")], |_| true);
    let json = serde_json::to_value(status(&[], &proposal)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "state": "not-installed",
            "folder": "/Users/ann/.local/bin",
            "path": "/Users/ann/.local/bin/anima",
            "onPath": true,
            "create": false,
            "profileLine": null,
        })
    );
    let unavailable = Status::Unavailable {
        reason: CommandLineError::DiskImage.code(),
    };
    assert_eq!(
        serde_json::to_value(unavailable).unwrap(),
        serde_json::json!({ "state": "unavailable", "reason": "disk-image" })
    );
}

#[test]
fn ins_006_a_refusal_names_what_is_in_the_way() {
    let link = CommandLineError::Occupied(Occupant::Link(PathBuf::from("/opt/x/anima")));
    let json = serde_json::to_value(&link).unwrap();
    assert_eq!(json["code"], "occupied");
    assert_eq!(json["occupant"], "link");
    assert_eq!(json["linkTarget"], "/opt/x/anima");
    let file = serde_json::to_value(CommandLineError::Occupied(Occupant::File)).unwrap();
    assert_eq!(file["occupant"], "file");
    assert!(file.get("linkTarget").is_none());
}

#[test]
fn ins_005_only_the_flag_first_starts_the_command_line() {
    let args = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
    let started = args(&[
        "anima-desktop",
        "--anima-cli",
        "device",
        "list",
        "--anima-cli",
    ]);
    assert_eq!(cli_args(&started), Some(&started[2..]));
    let bare = args(&["anima-desktop", "--anima-cli"]);
    assert_eq!(cli_args(&bare), Some(&bare[2..]));
    for ordinary in [
        args(&["anima-desktop"]),
        args(&["anima-desktop", "anima://open"]),
        args(&["anima-desktop", "x", "--anima-cli"]),
    ] {
        assert_eq!(cli_args(&ordinary), None);
    }
}

#[test]
fn ins_005_relative_paths_are_read_from_the_folder_the_command_was_typed_in() {
    assert_eq!(
        typed_in(Some(OsStr::new("/home/ann/work"))),
        Some(PathBuf::from("/home/ann/work"))
    );
    assert_eq!(typed_in(Some(OsStr::new("work"))), None);
    assert_eq!(typed_in(None), None);
}

#[test]
fn ins_004_the_login_path_is_the_marked_line_whatever_the_profile_prints() {
    let output = "Welcome!\nanima-path:/fake\nmore\nanima-path:/usr/bin:/Users/ann/.local/bin\n";
    assert_eq!(path_line(output), Some("/usr/bin:/Users/ann/.local/bin"));
    assert_eq!(path_line("no mark\n"), None);
}
