//! The release sources of the runtimes and the versions that this program accepts (Section 6,
//! CAG-008, SEC-C-005). The operator writes the sources at the machine in `model-sources.json`
//! of the daemon folder; the hub cannot write them. The lowest versions and the list of
//! versions with an advisory are in this program, and only a signed release changes them.

use serde::Deserialize;

use super::cause::Cause;
use super::names::{Backend, Runtime, Word};

/// The lowest release of each runtime that this program accepts. The number of
/// `llama_server` is a placeholder until the human check of HT-NEW-l1daemonwork picks the
/// first release of each machine (OD-C1).
pub const LOWEST: [(Runtime, u64); 1] = [(Runtime::LlamaServer, 6000)];

/// The releases with a published advisory (SEC-C-005). Empty in this release.
pub const ADVISORIES: [(Runtime, u64); 0] = [];

/// The most bytes of one archive that a source entry may name (8 GiB).
pub const MAX_ARCHIVE_BYTES: u64 = 8 << 30;

/// A release of a runtime that an item names: the runtime, its version and the backend.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Release {
    pub runtime: Runtime,
    pub version: String,
    pub backend: Backend,
}

impl Release {
    /// The number of the version, by the pattern of its runtime: for `llama_server`, `b` and
    /// at most 9 digits. The other runtimes have no pattern yet (open point MW-P4), so no
    /// version of them reads.
    #[must_use]
    pub fn number(runtime: Runtime, version: &str) -> Option<u64> {
        let digits = match runtime {
            Runtime::LlamaServer => version.strip_prefix('b')?,
            Runtime::MlxLm | Runtime::Vllm => return None,
        };
        let plain = (1..=9).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit());
        plain.then(|| digits.parse().ok()).flatten()
    }

    /// The name of its folder below `runtimes/<runtime>/`: `<version>-<backend>`.
    #[must_use]
    pub fn folder(&self) -> String {
        format!("{}-{}", self.version, self.backend.as_str())
    }
}

/// The check of the program lists: `runtime_too_old`, then `runtime_advisory`.
///
/// # Errors
/// The cause of the first list that refuses the release.
pub fn accepted(release: &Release) -> Result<(), Cause> {
    let number = Release::number(release.runtime, &release.version).ok_or(Cause::MemberUnknown)?;
    let lowest = LOWEST
        .iter()
        .find(|(runtime, _)| *runtime == release.runtime);
    if lowest.is_none_or(|(_, lowest)| number < *lowest) {
        return Err(Cause::RuntimeTooOld);
    }
    if ADVISORIES.contains(&(release.runtime, number)) {
        return Err(Cause::RuntimeAdvisory);
    }
    Ok(())
}

/// The form of a runtime archive, by the end of its address (R-NEW-archivezip).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

/// One entry of `model-sources.json`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceEntry {
    pub runtime: String,
    pub version: String,
    pub os: String,
    pub architecture: String,
    pub backend: String,
    /// The HTTPS address of the archive (a `.tar.gz`, `.tgz` or `.zip`).
    pub address: String,
    pub sha256: String,
    pub size_bytes: u64,
    /// The path of the server program inside the archive, as `build/bin/llama-server`.
    pub program: String,
}

/// The file `model-sources.json`: `{ "sources": [ … ] }`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub sources: Vec<SourceEntry>,
}

impl SourceEntry {
    /// The form of the archive, from the end of the path of its address; `None` for another.
    #[must_use]
    pub fn format(&self) -> Option<ArchiveFormat> {
        let path = std::path::Path::new(self.address.split(['?', '#']).next().unwrap_or_default());
        let is = |found: Option<&std::ffi::OsStr>, wanted: &str| {
            found.is_some_and(|found| found.eq_ignore_ascii_case(wanted))
        };
        let inner = path.file_stem().map(std::path::Path::new);
        if is(path.extension(), "tgz")
            || (is(path.extension(), "gz") && is(inner.and_then(std::path::Path::extension), "tar"))
        {
            Some(ArchiveFormat::TarGz)
        } else if is(path.extension(), "zip") {
            Some(ArchiveFormat::Zip)
        } else {
            None
        }
    }
}

fn entry_holds(entry: &SourceEntry) -> bool {
    entry.format().is_some()
        && entry.address.starts_with("https://")
        && super::names::is_lower_hex(&entry.sha256, 64)
        && (1..=MAX_ARCHIVE_BYTES).contains(&entry.size_bytes)
        && super::files::is_file_path(&entry.program)
}

impl Sources {
    /// The entry for `release` on this operating system and architecture.
    ///
    /// # Errors
    /// `runtime_not_configured` when no entry names it, or the entry is not whole.
    pub fn find(
        &self,
        release: &Release,
        os: &str,
        architecture: &str,
    ) -> Result<&SourceEntry, Cause> {
        self.sources
            .iter()
            .find(|entry| {
                entry.runtime == release.runtime.as_str()
                    && entry.version == release.version
                    && entry.backend == release.backend.as_str()
                    && entry.os == os
                    && entry.architecture == architecture
            })
            .filter(|entry| entry_holds(entry))
            .ok_or(Cause::RuntimeNotConfigured)
    }
}
