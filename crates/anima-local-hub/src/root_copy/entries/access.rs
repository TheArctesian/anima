//! Private copied state uses Unix modes or Windows ACLs. Reparse points are never followed.
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, Metadata},
    io,
    path::Path,
};

pub(super) fn mode(found: &Metadata) -> u32 {
    #[cfg(unix)]
    return found.permissions().mode() & 0o7777;
    #[cfg(windows)]
    if found.is_dir() {
        0o700
    } else if found.permissions().readonly() {
        0o400
    } else {
        0o600
    }
}

#[cfg_attr(
    unix,
    expect(
        clippy::unnecessary_wraps,
        reason = "Windows inspects the endpoint ACL; callers share the fallible boundary"
    )
)]
pub(super) fn socket(path: &Path, found: &Metadata) -> io::Result<bool> {
    #[cfg(unix)]
    {
        let _ = path;
        Ok(found.file_type().is_socket())
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if found.file_attributes() & 0x400 == 0 {
            return Ok(false);
        }
        if anima_windows::socket::private(path)? {
            return Ok(true);
        }
        Err(io::ErrorKind::PermissionDenied.into())
    }
}

pub(super) fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    #[cfg(unix)]
    return fs::set_permissions(path, fs::Permissions::from_mode(mode));
    #[cfg(windows)]
    {
        anima_windows::files::protect(path)?;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(mode & 0o200 == 0);
        fs::set_permissions(path, permissions)
    }
}

pub(super) fn open(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    return File::open(path);
    #[cfg(windows)]
    anima_windows::files::read(path, true)
}

pub(super) fn create(path: &Path, mode: u32) -> io::Result<File> {
    #[cfg(unix)]
    return fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode & 0o700)
        .open(path);
    #[cfg(windows)]
    {
        let _ = mode;
        anima_windows::files::create_new(path)
    }
}

pub(super) fn symlink(target: &Path, to: &Path) -> io::Result<()> {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(target, to);
    #[cfg(windows)]
    {
        let _ = (target, to);
        Err(io::ErrorKind::Unsupported.into())
    }
}

#[cfg(windows)]
pub(super) fn verify(path: &Path) -> io::Result<()> {
    use std::os::windows::fs::MetadataExt;
    let found = fs::symlink_metadata(path)?;
    if found.file_attributes() & 0x400 != 0 || !anima_windows::files::secure(path, true)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    for ancestor in path.ancestors().skip(1) {
        if !anima_windows::files::secure(ancestor, false)? {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
    }
    Ok(())
}
