//! Platform permissions for a carried program tree. Windows bundles contain real files,
//! never junctions or symlinks; new copies inherit their private destination directory.
#[cfg(unix)]
use std::{
    fs::OpenOptions,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
};
use std::{
    fs::{self, File},
    io,
    path::Path,
};

pub(super) fn source(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use nix::fcntl::OFlag;
        OpenOptions::new()
            .read(true)
            .custom_flags((OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK).bits())
            .open(path)
    }
    #[cfg(windows)]
    anima_windows::files::read(path, false)
}

pub(super) fn folder(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    return fs::DirBuilder::new().mode(0o700).create(path);
    #[cfg(windows)]
    anima_database::owner_only::folder(path)
}

pub(super) fn create(path: &Path, mode: u32) -> io::Result<File> {
    #[cfg(unix)]
    return OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path);
    #[cfg(windows)]
    {
        let _ = mode; // Windows execution is determined by the image, not Unix mode bits.
        anima_windows::files::create_new(path)
    }
}

pub(super) fn set_mode(file: &File, mode: u32) -> io::Result<()> {
    #[cfg(unix)]
    return file.set_permissions(fs::Permissions::from_mode(mode));
    #[cfg(windows)]
    {
        let _ = mode;
        // Verify the handle still names the regular file created under our private parent.
        if !file.metadata()?.is_file() {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }
}

pub(super) fn executable(path: &Path, found: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        let _ = path;
        found.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        found.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    }
}

#[cfg(windows)]
pub(super) fn inspect_folder(path: &Path) -> io::Result<()> {
    let found = fs::symlink_metadata(path)?;
    if !found.is_dir() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    #[cfg(windows)]
    anima_windows::files::directory(path)?;
    Ok(())
}
