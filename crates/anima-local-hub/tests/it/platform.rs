//! Real executables and process controls on the host that runs the integration tests.
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
};

pub(super) fn program(folder: &Path, name: &str) -> PathBuf {
    let path = folder.join(name);
    #[cfg(windows)]
    return path.with_extension("exe");
    #[cfg(unix)]
    path
}

pub(super) fn environment(command: &mut Command) {
    command.env_clear();
    #[cfg(windows)]
    for name in [
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "LOCALAPPDATA",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

pub(super) fn programs(root: &Path, built: &Path) -> PathBuf {
    let folder = root.join("programs");
    if folder.is_dir() {
        return folder;
    }
    anima_database::owner_only::folder(&folder).unwrap();
    for name in [
        "anima-local-hub",
        "anima-hub",
        "anima-database",
        "anima-gateway",
    ] {
        #[cfg(unix)]
        std::os::unix::fs::symlink(program(built, name), program(&folder, name)).unwrap();
        #[cfg(windows)]
        std::fs::copy(program(built, name), program(&folder, name)).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(folder.join("anima-link"), super::LINK).unwrap();
        std::fs::set_permissions(
            folder.join("anima-link"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }
    #[cfg(windows)]
    {
        let fixture =
            std::env::var_os("ANIMA_LINK_FIXTURE_EXE").expect("build the offline link fixture");
        std::fs::copy(fixture, program(&folder, "anima-link")).unwrap();
    }
    folder
}

pub(super) fn stop(child: &mut Child) {
    #[cfg(unix)]
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(child.id()).unwrap()),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    #[cfg(windows)]
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(b"stop\n").unwrap();
    }
}

pub(super) fn assert_socket(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        assert!(
            std::fs::symlink_metadata(path)
                .unwrap()
                .file_type()
                .is_socket()
        );
    }
    #[cfg(windows)]
    assert!(anima_windows::socket::private(path).unwrap());
}
