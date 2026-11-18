//! The records of the files domain and the checks of their text (K-E4 Sections 9 and 10).

use chrono::{DateTime, Utc};

use super::location::Position;
use crate::{
    engine::records::Reference,
    ids::{ArchiveId, AttachmentId},
};

/// The largest file, counted while it streams (FIL-005): 256 MB.
pub const MAX_FILE: u64 = 268_435_456;
/// The longest file or archive name, in characters.
const MAX_NAME: usize = 200;
/// The longest media type, in bytes.
const MAX_MEDIA_TYPE: usize = 100;
/// What to give for a name of a file or an archive.
pub const NAME_RULE: &str = "Give one line of 1 to 200 characters.";
/// The media type of a file whose type was not given.
pub const UNKNOWN_TYPE: &str = "application/octet-stream";

/// A stored file of one record.
#[derive(Clone, Debug, PartialEq)]
pub struct Attachment {
    pub id: AttachmentId,
    pub owner: Reference,
    pub name: String,
    pub media_type: String,
    /// The SHA-256 of the plain bytes, in lower-case hexadecimal.
    pub hash: String,
    /// The data key that seals the piece.
    pub key_name: String,
    pub size: u64,
    pub archive: Option<ArchiveId>,
    /// Kept only when the operator asked for it at upload (FIL-003).
    pub position: Option<Position>,
    pub created_at: DateTime<Utc>,
}

/// The unit of deletion of one import (FIL-004).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Archive {
    pub id: ArchiveId,
    pub owner: Reference,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// What a delete removed, for its audit entries and for the pieces to remove after it.
#[derive(Debug, Default)]
pub struct Removed {
    pub attachments: Vec<Attachment>,
    /// `(key name, hash)` of each piece that no attachment uses now.
    pub pieces: Vec<(String, String)>,
    /// The records each `Derived` deleted, by its name.
    pub derived: Vec<(&'static str, u64)>,
}

/// A name of a file or an archive: trimmed, one line, 1 to 200 characters.
#[must_use]
pub fn name(text: &str) -> Option<String> {
    let text = text.trim();
    let fits = !text.is_empty() && text.chars().count() <= MAX_NAME;
    (fits && !text.chars().any(char::is_control)).then(|| text.to_owned())
}

/// The essence of a media type in lower case, such as `image/jpeg`; `None` when it is not
/// `type/subtype` of token characters. No type is [`UNKNOWN_TYPE`].
#[must_use]
pub fn media_type(text: Option<&str>) -> Option<String> {
    let Some(text) = text else {
        return Some(UNKNOWN_TYPE.to_owned());
    };
    let essence = text.split(';').next().unwrap_or_default().trim();
    let token = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&-^_.+".contains(&b))
    };
    let (kind, subtype) = essence.split_once('/')?;
    (essence.len() <= MAX_MEDIA_TYPE && token(kind) && token(subtype))
        .then(|| essence.to_ascii_lowercase())
}
