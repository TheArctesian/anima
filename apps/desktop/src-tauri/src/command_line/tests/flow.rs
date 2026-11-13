//! Install and Remove in a temporary HOME (INS-004 to INS-006): a stand-in app bundle or `AppImage`,
//! and the real link, launcher and file checks. No real `PATH` folder is touched.

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

use crate::command_line::{
    CommandLineError, Here, carrier::Carrier, error::Occupant, install_in, launcher, remove_all,
    state::Status,
};

struct Rig {
    _dir: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Rig {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let home = root.join("home");
        fs::create_dir(&home).unwrap();
        Self {
            _dir: dir,
            root,
            home,
        }
    }

    fn local_bin(&self) -> PathBuf {
        self.home.join(".local/bin")
    }

    /// An app bundle whose `anima` prints `bundled` and its arguments.
    fn bundle(&self, name: &str) -> Carrier {
        let macos = self.root.join(name).join("Contents/MacOS");
        fs::create_dir_all(&macos).unwrap();
        let program = macos.join("anima");
        fs::write(&program, "#!/bin/sh\necho bundled \"$@\"\n").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        Carrier::Bundle { program }
    }

    fn here(&self, carrier: Carrier, on_path: bool) -> Here {
        let path = if on_path {
            vec![self.local_bin()]
        } else {
            Vec::new()
        };
        Here {
            carrier,
            home: self.home.clone(),
            path,
        }
    }
}

fn target(path: &Path) -> PathBuf {
    fs::read_link(path).unwrap()
}

#[test]
fn ins_004_install_links_anima_into_the_app_and_remove_deletes_the_link() {
    let rig = Rig::new();
    fs::create_dir_all(rig.local_bin()).unwrap();
    let carrier = rig.bundle("Anima.app");
    let here = rig.here(carrier.clone(), true);
    let link = rig.local_bin().join("anima");
    install_in(&here, &rig.local_bin()).unwrap();
    let Carrier::Bundle { program } = &carrier else {
        unreachable!()
    };
    assert_eq!(&target(&link), program);
    let ran = Command::new(&link).arg("version").output().unwrap();
    assert_eq!(String::from_utf8(ran.stdout).unwrap(), "bundled version\n");
    assert_eq!(
        here.status(),
        Status::Installed {
            path: link.to_string_lossy().into_owned()
        }
    );
    install_in(&here, &rig.local_bin()).expect("installing again changes nothing");
    remove_all(&here).unwrap();
    assert!(fs::symlink_metadata(&link).is_err());
    assert!(program.is_file(), "the program in the app stays");
    assert!(matches!(here.status(), Status::NotInstalled { .. }));
}

#[test]
fn ins_009_with_no_folder_on_the_path_local_bin_is_made_with_mode_0755() {
    let rig = Rig::new();
    let here = rig.here(rig.bundle("Anima.app"), false);
    let Status::NotInstalled {
        create,
        profile_line,
        ..
    } = here.status()
    else {
        panic!("not installed");
    };
    assert!(create);
    assert!(profile_line.is_some());
    install_in(&here, &rig.local_bin()).unwrap();
    let mode = fs::metadata(rig.local_bin()).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    assert!(matches!(here.status(), Status::Installed { .. }));
}

#[test]
fn ins_006_a_file_or_link_the_app_did_not_make_is_never_replaced_or_removed() {
    let rig = Rig::new();
    fs::create_dir_all(rig.local_bin()).unwrap();
    let here = rig.here(rig.bundle("Anima.app"), true);
    let path = rig.local_bin().join("anima");
    fs::write(&path, "someone else's anima").unwrap();
    assert_eq!(
        install_in(&here, &rig.local_bin()),
        Err(CommandLineError::Occupied(Occupant::File))
    );
    remove_all(&here).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "someone else's anima");
    fs::remove_file(&path).unwrap();
    let other = rig.root.join("other-tool");
    symlink(&other, &path).unwrap();
    assert_eq!(
        install_in(&here, &rig.local_bin()),
        Err(CommandLineError::Occupied(Occupant::Link(other.clone())))
    );
    remove_all(&here).unwrap();
    assert_eq!(target(&path), other);
}

#[test]
fn ins_006_a_link_to_another_anima_app_is_shown_and_pointed_at_this_app() {
    let rig = Rig::new();
    fs::create_dir_all(rig.local_bin()).unwrap();
    let Carrier::Bundle { program: old } = rig.bundle("Old Anima.app") else {
        unreachable!()
    };
    let path = rig.local_bin().join("anima");
    symlink(&old, &path).unwrap();
    let carrier = rig.bundle("Anima.app");
    let here = rig.here(carrier.clone(), true);
    assert!(
        matches!(here.status(), Status::Elsewhere { points_at, .. } if Path::new(&points_at) == old)
    );
    install_in(&here, &rig.local_bin()).unwrap();
    let Carrier::Bundle { program } = carrier else {
        unreachable!()
    };
    assert_eq!(target(&path), program);
}

#[test]
fn ins_004_only_the_proposed_folder_or_an_anima_entrys_folder_is_written() {
    let rig = Rig::new();
    fs::create_dir_all(rig.local_bin()).unwrap();
    let here = rig.here(rig.bundle("Anima.app"), true);
    for folder in [
        rig.home.join("bin"),
        rig.root.clone(),
        PathBuf::from("/etc"),
    ] {
        assert_eq!(
            install_in(&here, &folder),
            Err(CommandLineError::Changed),
            "{folder:?}"
        );
        assert!(fs::symlink_metadata(folder.join("anima")).is_err());
    }
}

#[test]
fn ins_005_install_writes_a_0755_launcher_for_the_appimage_and_remove_deletes_it() {
    let rig = Rig::new();
    fs::create_dir_all(rig.local_bin()).unwrap();
    let file = rig.root.join("Apps/Anima-x86_64.AppImage");
    let here = rig.here(Carrier::AppImage { file: file.clone() }, true);
    install_in(&here, &rig.local_bin()).unwrap();
    let path = rig.local_bin().join("anima");
    let mode = fs::symlink_metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(launcher::appimage_of(&text), Some(file));
    assert!(matches!(here.status(), Status::Installed { .. }));
    let moved = Carrier::AppImage {
        file: rig.root.join("Moved.AppImage"),
    };
    let after_move = rig.here(moved, true);
    assert!(matches!(after_move.status(), Status::Elsewhere { .. }));
    remove_all(&after_move).unwrap();
    assert!(fs::symlink_metadata(&path).is_err());
    let leftovers: Vec<_> = fs::read_dir(rig.local_bin()).unwrap().collect();
    assert!(leftovers.is_empty(), "no part file is left");
}
