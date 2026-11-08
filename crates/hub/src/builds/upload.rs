//! The rules of an `uploaded` report (BLD-063, D-5): the file list and the size and form of
//! the proposed feed. Publishing checks the feed's content (task 3).
//!
//! Pure.

use std::collections::HashSet;

use super::model::{FieldError, UploadedFile};

/// The most files one job uploads: the updater file, the DMG and `SHA256SUMS`, with room.
const MAX_FILES: usize = 8;

/// The largest proposed feed in bytes; `latest.json` is a few hundred.
const MAX_FEED_BYTES: usize = 64 * 1024;

/// A file name: a letter, digit or `_` first, then letters, digits, `.`, `_` or `-`, at most
/// 128 bytes. No path separator and no leading dot.
fn file_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    let allowed = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-');
    !bytes.is_empty()
        && bytes.len() <= 128
        && (bytes[0].is_ascii_alphanumeric() || bytes[0] == b'_')
        && bytes.iter().copied().all(allowed)
}

/// A SHA-256 as 64 hex digits, lowered.
fn sha256(text: &str) -> Option<String> {
    (text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| text.to_ascii_lowercase())
}

/// The reported files, checked: 1 to 8 of them, each name valid and named once ignoring ASCII
/// case, each SHA-256 64 hex digits and each size above zero.
pub(super) fn files(reported: &[UploadedFile]) -> Result<Vec<UploadedFile>, FieldError> {
    if reported.is_empty() || reported.len() > MAX_FILES {
        return Err(FieldError::new(
            "files",
            format!("Report 1 to {MAX_FILES} files."),
        ));
    }
    let mut names = HashSet::new();
    reported
        .iter()
        .map(|file| {
            // Compared without case: on a case-insensitive disk or bucket view two such
            // names are one file (T3).
            if !file_name(&file.name) || !names.insert(file.name.to_ascii_lowercase()) {
                return Err(FieldError::new(
                    "files",
                    "Each file needs one plain file name, used once.",
                ));
            }
            let sha256 = sha256(&file.sha256)
                .ok_or_else(|| FieldError::new("files", "Each SHA-256 is 64 hex digits."))?;
            if file.size <= 0 {
                return Err(FieldError::new("files", "Each size is above zero."));
            }
            Ok(UploadedFile {
                name: file.name.clone(),
                sha256,
                size: file.size,
            })
        })
        .collect()
}

/// The proposed feed, checked: a JSON object of at most 64 KiB, kept as the hub writes it out
/// again, so what is stored is exactly what the hub read (a repeated key keeps its last value,
/// and no stray text survives). Written out, it can be longer than the text it came from
/// (`1E2` becomes `100.0`), so the 64 KiB limit applies to both.
pub(super) fn feed(text: &str) -> Result<String, FieldError> {
    let refused = || FieldError::new("feed", "The feed is a JSON object of at most 64 KiB.");
    if text.len() > MAX_FEED_BYTES {
        return Err(refused());
    }
    let Ok(object @ serde_json::Value::Object(_)) = serde_json::from_str(text) else {
        return Err(refused());
    };
    serde_json::to_string(&object)
        .ok()
        .filter(|written| written.len() <= MAX_FEED_BYTES)
        .ok_or_else(refused)
}
