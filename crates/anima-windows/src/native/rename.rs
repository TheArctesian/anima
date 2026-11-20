//! Publish a flushed private file with the Windows write-through rename operation.
use std::{io, os::windows::ffi::OsStrExt, path::Path};
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
    core::PCWSTR,
};

pub(crate) fn flushed(from: &Path, to: &Path) -> io::Result<()> {
    fn wide(path: &Path) -> io::Result<Vec<u16>> {
        let mut text: Vec<_> = path.as_os_str().encode_wide().collect();
        if text.contains(&0) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        text.push(0);
        Ok(text)
    }
    let from = wide(from)?;
    let to = wide(to)?;
    // SAFETY: both terminated paths outlive the call; no cross-volume copy is permitted.
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(io::Error::other)
}
