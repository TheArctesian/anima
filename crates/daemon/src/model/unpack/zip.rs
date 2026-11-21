//! The entries of a `.zip` (R-NEW-archivezip), with the rules of the `.tar.gz`: only a regular
//! file or a folder (the Unix type bits, when the entry has them, say which), no symbolic link,
//! no encrypted entry, the declared size and the bytes read both within the limit, so a zip
//! bomb stops at the limit.

use std::fs::File;

use daemon_core::model_work::cause::Cause;

use super::{BAD_ARCHIVE, Kind, Writer};

/// The type bits of a Unix mode, and the two types that an entry may have.
const TYPE_BITS: u32 = 0o170_000;
const REGULAR: u32 = 0o100_000;
const FOLDER: u32 = 0o040_000;

pub(super) fn unpack<F: Fn() -> bool>(archive: File, writer: &mut Writer<F>) -> Result<(), Cause> {
    let mut zip = ::zip::ZipArchive::new(archive).map_err(|_| BAD_ARCHIVE)?;
    if zip.len() > writer.max_entries {
        return Err(Cause::LimitDisk);
    }
    for index in 0..zip.len() {
        writer.next_entry()?;
        let mut entry = zip.by_index(index).map_err(|_| BAD_ARCHIVE)?;
        let mode = entry.unix_mode().unwrap_or(0);
        let type_bits = mode & TYPE_BITS;
        if entry.is_symlink() || entry.encrypted() || ![0, REGULAR, FOLDER].contains(&type_bits) {
            return Err(BAD_ARCHIVE);
        }
        if entry.size() > writer.left {
            return Err(Cause::LimitDisk);
        }
        let folder = entry.is_dir() || type_bits == FOLDER;
        let kind = if folder {
            Kind::Folder
        } else {
            Kind::File(mode)
        };
        let path = entry.name().map_err(|_| BAD_ARCHIVE)?.into_owned();
        writer.write(&path, kind, &mut entry)?;
    }
    Ok(())
}
