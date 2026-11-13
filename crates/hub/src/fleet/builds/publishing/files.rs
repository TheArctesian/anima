//! The files of one job as the hub checks them (BLD-061, BLD-063, BLD-064, UPD-010): which names
//! a job may upload and as which content type, which must be there, which one gets the stable
//! name, the bytes read back from the bucket, and the content of `SHA256SUMS`. Pure: the bytes
//! come in chunks from the caller.

use sha2::{Digest as _, Sha256};

use super::{feed::SUMS, sigv4::hex};
use crate::fleet::builds::{
    model::{Target, UploadedFile},
    version::Version,
};

/// The `Content-Type` of `name` when it is a file a job of `target` and `version` uploads: the
/// updater file, the DMG on macOS, or `SHA256SUMS`; `None` for any other name (BLD-061, BLD-063).
/// The upload URL signs it, so a builder cannot publish a page or a script.
pub(super) fn content_type(target: Target, version: Version, name: &str) -> Option<&'static str> {
    match target {
        _ if name == SUMS => Some("text/plain; charset=utf-8"),
        Target::LinuxX86_64 => (name == format!("Anima_{version}_amd64.AppImage"))
            .then_some("application/vnd.appimage"),
        Target::DarwinUniversal if name == "Anima.app.tar.gz" => Some("application/gzip"),
        Target::DarwinUniversal => (name == format!("Anima_{version}_universal.dmg"))
            .then_some("application/x-apple-diskimage"),
    }
}

/// The largest `SHA256SUMS` the hub reads: one line of at most 196 bytes per file.
const MAX_SUMS_BYTES: i64 = 4096;

/// The stable name for people of `target`'s file that ends with the returned suffix (UPD-010).
const fn stable(target: Target) -> (&'static str, &'static str) {
    match target {
        Target::LinuxX86_64 => (".AppImage", "Anima-x86_64.AppImage"),
        Target::DarwinUniversal => (".dmg", "Anima.dmg"),
    }
}

/// The copy the hub makes for people: the job's one file with the target's ending, and its
/// stable name (BLD-066). Refused unless `SHA256SUMS` is there too, small enough to read.
pub(super) fn stable_copy(
    target: Target,
    files: &[UploadedFile],
) -> Result<(String, &'static str), String> {
    // The bytes of SHA256SUMS are kept in memory as they are read; the limit bounds them.
    let sums_ok = files
        .iter()
        .any(|file| file.name == SUMS && file.size <= MAX_SUMS_BYTES);
    if !sums_ok {
        return Err(format!(
            "The job has no {SUMS} of at most {MAX_SUMS_BYTES} bytes."
        ));
    }
    let (suffix, name) = stable(target);
    let mut matching = files.iter().filter(|file| file.name.ends_with(suffix));
    match (matching.next(), matching.next()) {
        (Some(file), None) => Ok((file.name.clone(), name)),
        _ => Err(format!("The job needs exactly one {suffix} file.")),
    }
}

/// Checks `SHA256SUMS`: one `<sha256>  <name>` line for each other file of the job, with the
/// digest the builder reported, and nothing else (UPD-010).
pub(super) fn check_sums(bytes: &[u8], files: &[UploadedFile]) -> Result<(), String> {
    let refused = || format!("{SUMS} does not list each file of the job with its SHA-256.");
    let text = std::str::from_utf8(bytes).map_err(|_| refused())?;
    let mut listed: Vec<(&str, &str)> = text
        .lines()
        .map(|line| line.split_once("  ").ok_or_else(refused))
        .map(|pair| pair.map(|(sha256, name)| (name, sha256)))
        .collect::<Result<_, _>>()?;
    listed.sort_unstable();
    let mut wanted: Vec<(&str, &str)> = files
        .iter()
        .filter(|file| file.name != SUMS)
        .map(|file| (file.name.as_str(), file.sha256.as_str()))
        .collect();
    wanted.sort_unstable();
    (listed == wanted).then_some(()).ok_or_else(refused)
}

/// One file as it comes back from the public address, checked as it streams: never more bytes
/// than reported, then the reported SHA-256 (BLD-064). Keeps the bytes only of `SHA256SUMS`.
pub(super) struct Received<'a> {
    file: &'a UploadedFile,
    read: i64,
    hash: Sha256,
    kept: Option<Vec<u8>>,
}

impl<'a> Received<'a> {
    pub(super) fn new(file: &'a UploadedFile) -> Self {
        Self {
            file,
            read: 0,
            hash: Sha256::new(),
            kept: (file.name == SUMS).then(Vec::new),
        }
    }

    /// Takes the next chunk; refused once the file is longer than reported.
    pub(super) fn push(&mut self, chunk: &[u8]) -> Result<(), String> {
        let length = i64::try_from(chunk.len()).unwrap_or(i64::MAX);
        self.read = self.read.saturating_add(length);
        if self.read > self.file.size {
            return Err(self.wrong_size());
        }
        self.hash.update(chunk);
        if let Some(kept) = &mut self.kept {
            kept.extend_from_slice(chunk);
        }
        Ok(())
    }

    fn wrong_size(&self) -> String {
        format!(
            "{} is not the {} bytes the builder reported.",
            self.file.name, self.file.size
        )
    }

    /// The kept bytes once the file had exactly the reported size and SHA-256.
    pub(super) fn finish(self) -> Result<Vec<u8>, String> {
        if self.read != self.file.size {
            return Err(self.wrong_size());
        }
        if hex(&self.hash.finalize()) != self.file.sha256 {
            return Err(format!(
                "{} does not have the SHA-256 the builder reported.",
                self.file.name
            ));
        }
        Ok(self.kept.unwrap_or_default())
    }
}
