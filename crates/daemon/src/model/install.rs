//! `runtime_install` (Section 6, CAG-008): the release source of the configuration, the
//! SHA-256 verified before the archive is opened, the unpack into a new temporary folder, then
//! one rename to `runtimes/<runtime>/<version>-<backend>/`. No shell, no install script, no
//! package manager.

use std::io::{Seek as _, SeekFrom};
use std::path::Path;

use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::folders::Folders;
use daemon_core::model_work::names::Word as _;
use daemon_core::model_work::releases::{self, Release, SourceEntry, Sources};

use super::{fetch, folders, unpack};

/// Where the machine is and what its operator configured.
pub struct Place<'a> {
    pub folders: &'a Folders,
    pub sources: &'a Sources,
    pub os: &'a str,
    pub architecture: &'a str,
}

/// Whether `release` is installed: its folder holds the program of its entry as a regular file.
#[must_use]
pub fn installed(folders: &Folders, release: &Release, entry: &SourceEntry) -> bool {
    let program = folders.release(release).join(&entry.program);
    std::fs::symlink_metadata(program).is_ok_and(|meta| meta.is_file())
}

/// Installs `release`.
///
/// # Errors
/// The cause of Section 5 for the first step that fails.
pub async fn install(release: &Release, place: &Place<'_>) -> Result<(), Cause> {
    releases::accepted(release)?;
    let entry = place.sources.find(release, place.os, place.architecture)?;
    if installed(place.folders, release, entry) {
        return Ok(());
    }
    folders::make_all(place.folders).map_err(|_| Cause::LimitDisk)?;
    let mut archive = tempfile::tempfile_in(&place.folders.state).map_err(|_| Cause::LimitDisk)?;
    let hosts = fetch::release_hosts(release.runtime);
    let sha = fetch::fetch(&entry.address, hosts, entry.size_bytes, &mut archive).await?;
    if sha != entry.sha256 {
        return Err(Cause::Checksum);
    }
    archive
        .seek(SeekFrom::Start(0))
        .map_err(|_| Cause::LimitDisk)?;
    place_archive(archive, place.folders, release, entry)
}

/// Unpacks a verified `archive` and renames its folder into place.
///
/// # Errors
/// As [`unpack::unpack`]; `archive_rejected` when the archive has no program at the path of
/// its entry.
pub fn place_archive(
    archive: std::fs::File,
    folders: &Folders,
    release: &Release,
    entry: &SourceEntry,
) -> Result<(), Cause> {
    let parent = folders.runtimes.join(release.runtime.as_str());
    folders::private_folder(&parent).map_err(|_| Cause::LimitDisk)?;
    let temporary = tempfile::Builder::new()
        .prefix(".unpack-")
        .tempdir_in(&parent)
        .map_err(|_| Cause::LimitDisk)?;
    unpack::unpack(archive, temporary.path())?;
    let program = temporary.path().join(&entry.program);
    if !std::fs::symlink_metadata(&program).is_ok_and(|meta| meta.is_file()) {
        return Err(Cause::ArchiveRejected);
    }
    let target = folders.release(release);
    rename_into(temporary.path(), &target)?;
    // The folder now has its final name; nothing is left for the guard to delete.
    let _ = temporary.keep();
    Ok(())
}

fn rename_into(from: &Path, to: &Path) -> Result<(), Cause> {
    std::fs::rename(from, to).map_err(|_| Cause::LimitDisk)
}
