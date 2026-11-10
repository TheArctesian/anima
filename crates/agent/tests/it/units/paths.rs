//! `anima-agent install` refuses a binary or a folder another user could change, and a binary
//! the unit's sandbox hides, before it writes anything (MET-019).

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use anima_agent::units::folders::FolderError;
use anima_agent::units::install::{Host, InstallError, install};
use anima_agent::units::plan::Target;

use super::rig::enrolled;

#[test]
fn met_019_install_refuses_a_binary_in_a_folder_other_users_can_replace_in() {
    let rig = enrolled();
    let bin = rig.dir.path().join("bin");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o777)).expect("chmod");
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::Folder(FolderError::UntrustedFolder(folder)) if folder == bin.canonicalize().expect("path")
    ));
    assert!(!rig.unit().exists());
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_a_binary_other_users_can_write() {
    let rig = enrolled();
    fs::set_permissions(&rig.program, fs::Permissions::from_mode(0o777)).expect("chmod");
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::Folder(FolderError::UntrustedBinary(file)) if file == rig.program.canonicalize().expect("path")
    ));
    assert!(!rig.unit().exists());
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_a_unit_folder_that_links_to_one_others_can_write() {
    let rig = enrolled();
    let open = rig.dir.path().join("open");
    fs::create_dir(&open).expect("open");
    fs::set_permissions(&open, fs::Permissions::from_mode(0o777)).expect("chmod");
    fs::create_dir_all(rig.dir.path().join(".config/systemd")).expect("systemd");
    std::os::unix::fs::symlink(&open, rig.dir.path().join(".config/systemd/user")).expect("link");
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::Folder(FolderError::UntrustedFolder(folder)) if folder == open.canonicalize().expect("path")
    ));
    assert_eq!(
        fs::read_dir(&open).expect("open").count(),
        0,
        "nothing written"
    );
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_makes_no_folder_through_a_link_to_one_others_can_write() {
    let rig = enrolled();
    let open = rig.dir.path().join("open");
    fs::create_dir(&open).expect("open");
    fs::set_permissions(&open, fs::Permissions::from_mode(0o777)).expect("chmod");
    fs::create_dir(rig.dir.path().join(".config")).ok();
    std::os::unix::fs::symlink(&open, rig.dir.path().join(".config/systemd")).expect("link");
    let (result, ran) = rig.install(true);
    assert!(matches!(
        result.expect_err("refused"),
        InstallError::Folder(FolderError::UntrustedFolder(folder)) if folder == open.canonicalize().expect("path")
    ));
    assert_eq!(
        fs::read_dir(&open).expect("open").count(),
        0,
        "no `user` folder made"
    );
    assert_eq!(ran.len(), 0);
}

#[test]
fn met_019_install_refuses_a_binary_under_tmp_which_the_unit_s_sandbox_hides() {
    let rig = enrolled();
    let tmp = tempfile::tempdir_in("/tmp").expect("a folder in /tmp");
    let program = tmp.path().join("anima-agent");
    fs::copy(&rig.program, &program).expect("copy");
    let host = Host {
        program: &program,
        ..rig.host()
    };
    let result = install(Target::User, &host, rig.env(), |_| Ok(true));
    let error = result.expect_err("refused");
    assert!(matches!(&error, InstallError::Hidden { root: "/tmp", .. }));
    assert!(error.to_string().contains("move the binary out of /tmp"));
    assert!(!rig.unit().exists());
}

#[test]
fn met_019_install_refuses_a_binary_whose_path_systemd_would_misread() {
    let rig = enrolled();
    let odd = rig.dir.path().join("bin/anima agent%h");
    fs::rename(&rig.program, &odd).expect("rename");
    let host = Host {
        program: &odd,
        ..rig.host()
    };
    let result = install(Target::User, &host, rig.env(), |_| Ok(true));
    assert!(matches!(result, Err(InstallError::Program(_))));
    assert!(!rig.unit().exists());
}
