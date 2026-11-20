//! Inspect an `AF_UNIX` endpoint without following a junction or symbolic link.
pub mod transfer;
mod transport;
use std::{
    fs::OpenOptions,
    io,
    mem::size_of,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
pub use transport::{Listener, Stream};
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        FILE_ATTRIBUTE_TAG_INFO, FileAttributeTagInfo, GetFileInformationByHandleEx,
    },
};

/// Check a private socket and its directory chain before sending credentials.
/// # Errors
/// A path is missing, exposed, replaceable, or not a native Unix-domain endpoint.
pub fn trusted(path: &Path) -> io::Result<()> {
    let folder = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    private_folder(folder)?;
    if !private(path)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

fn private_folder(path: &Path) -> io::Result<()> {
    if !std::fs::symlink_metadata(path)?.is_dir() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    crate::files::directory(path)?;
    if !crate::files::secure(path, true)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

/// Whether the endpoint is the Windows `AF_UNIX` reparse type and has a private ACL.
/// # Errors
/// The endpoint or its ACL cannot be inspected. Missing sockets are reported as `NotFound`.
pub fn private(path: &Path) -> io::Result<bool> {
    let file = OpenOptions::new()
        .access_mode(0x0002_0080)
        .custom_flags(0x0020_0000)
        .open(path)?;
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: the borrowed file handle and writable correctly-sized buffer outlive the call.
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileAttributeTagInfo,
            (&raw mut attributes).cast(),
            u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>()).map_err(io::Error::other)?,
        )
    }
    .map_err(io::Error::other)?;
    // IO_REPARSE_TAG_AF_UNIX; do not accept symbolic links or directory junctions.
    Ok(attributes.ReparseTag == 0x8000_0023 && super::check(&file, true)?)
}
