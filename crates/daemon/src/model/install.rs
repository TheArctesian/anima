//! `runtime_install` (Section 6, CAG-008): the release source of the configuration, a check of
//! the free disk against the reserve, the download in a total time, the SHA-256 verified before
//! the archive is opened, the unpack on a blocking thread into a new temporary folder, then one
//! rename to `runtimes/<runtime>/<version>-<backend>/`. No shell, no install script, no package
//! manager.

use std::io::{Seek as _, SeekFrom};
use std::path::Path;
use std::time::Duration;

use daemon_core::model_work::cause::Cause;
use daemon_core::model_work::folders::Folders;
use daemon_core::model_work::names::Word as _;
use daemon_core::model_work::releases::{self, Release, SourceEntry, Sources};

use super::{fetch, folders, unpack};

/// The free disk that model work leaves to the machine (Section 7; default 10 GiB).
pub const DEFAULT_RESERVE: u64 = 10 << 30;

/// Where the machine is and what its operator configured.
pub struct Place<'a> {
    pub folders: &'a Folders,
    pub sources: &'a Sources,
    pub os: &'a str,
    pub architecture: &'a str,
    /// The disk reserve of the daemon configuration.
    pub reserve: u64,
}

/// What the unpack of one archive writes, and the reserve that it keeps.
#[derive(Clone)]
pub struct Target {
    pub folders: Folders,
    pub release: Release,
    pub entry: SourceEntry,
    pub reserve: u64,
}

/// Whether `release` is installed: its folder holds the program of its entry as a regular file.
#[must_use]
pub fn installed(folders: &Folders, release: &Release, entry: &SourceEntry) -> bool {
    let program = folders.release(release).join(&entry.program);
    std::fs::symlink_metadata(program).is_ok_and(|meta| meta.is_file())
}

/// The most time of one download: 5 minutes and one second for each MiB, at most 1 hour.
#[must_use]
pub fn fetch_deadline(size_bytes: u64) -> Duration {
    Duration::from_secs((300 + size_bytes / (1 << 20)).min(3_600))
}

/// Whether the file system of `path` has `need` bytes free above `reserve`.
fn fits(path: &Path, need: u64, reserve: u64) -> bool {
    folders::free_bytes(path).is_ok_and(|free| free >= need.saturating_add(reserve))
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
    let roots = [&place.folders.state, &place.folders.runtimes];
    if !roots
        .iter()
        .all(|root| fits(root, entry.size_bytes, place.reserve))
    {
        return Err(Cause::LimitDisk);
    }
    let mut archive = tempfile::tempfile_in(&place.folders.state).map_err(|_| Cause::LimitDisk)?;
    let hosts = fetch::release_hosts(release.runtime);
    let download = fetch::fetch(&entry.address, hosts, entry.size_bytes, &mut archive);
    let sha = tokio::time::timeout(fetch_deadline(entry.size_bytes), download)
        .await
        .map_err(|_| Cause::SourceFailed)??;
    if sha != entry.sha256 {
        return Err(Cause::Checksum);
    }
    archive
        .seek(SeekFrom::Start(0))
        .map_err(|_| Cause::LimitDisk)?;
    let target = Target {
        folders: place.folders.clone(),
        release: release.clone(),
        entry: entry.clone(),
        reserve: place.reserve,
    };
    tokio::task::spawn_blocking(move || place_archive(archive, &target))
        .await
        .map_err(|_| Cause::LimitDisk)?
}

/// Unpacks a verified `archive` and renames its folder into place. Blocking.
///
/// # Errors
/// As [`unpack::unpack`]; `archive_rejected` when the archive has no program at the path of
/// its entry.
pub fn place_archive(archive: std::fs::File, target: &Target) -> Result<(), Cause> {
    let runtime = target.release.runtime.as_str();
    folders::private_below(&target.folders.runtimes, Path::new(runtime))
        .map_err(|_| Cause::LimitDisk)?;
    let parent = target.folders.runtimes.join(runtime);
    let temporary = tempfile::Builder::new()
        .prefix(".unpack-")
        .tempdir_in(&parent)
        .map_err(|_| Cause::LimitDisk)?;
    let holds = || fits(&parent, 0, target.reserve);
    let format = target.entry.format().ok_or(Cause::RuntimeNotConfigured)?;
    unpack::unpack(archive, format, temporary.path(), holds)?;
    let program = temporary.path().join(&target.entry.program);
    if !std::fs::symlink_metadata(&program).is_ok_and(|meta| meta.is_file()) {
        return Err(Cause::ArchiveRejected);
    }
    std::fs::rename(temporary.path(), target.folders.release(&target.release))
        .map_err(|_| Cause::LimitDisk)?;
    // The folder now has its final name; nothing is left for the guard to delete.
    let _ = temporary.keep();
    Ok(())
}
