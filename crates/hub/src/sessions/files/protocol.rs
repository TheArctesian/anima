//! The files tunnel's text messages (K4 Section 9): the closed set the client may send, parsed
//! strictly, and the ones the hub sends. Pure.
//!
//! A client message with another `type`, a missing, repeated or extra field, a `size` that is
//! not a non-negative integer, a path that is empty or holds NUL, or more than
//! [`MAX_TEXT_BYTES`] in all is refused. The refusal names no part of the message (LOG-007).
//! A path is never longer than its message, so the message limit keeps every path under
//! K4's 4096 bytes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Largest text message from the client (K4 Section 9).
pub(super) const MAX_TEXT_BYTES: usize = 4 * 1024;
/// Largest path the hub handles, in bytes of UTF-8 (K4 Section 9): a client path is shorter
/// (its message is at most [`MAX_TEXT_BYTES`]); a longer path the device resolves is refused.
pub(super) const MAX_PATH_BYTES: usize = 4096;

/// The whitespace JSON allows between tokens (RFC 8259 Section 2).
const JSON_SPACE: [char; 4] = [' ', '\t', '\n', '\r'];

/// What the hub sends when it has sent nothing for a while.
pub(super) const PING: &str = r#"{"type":"ping"}"#;
/// What the hub sends when a transfer is over: all bytes moved, or cancelled.
pub(super) const DONE: &str = r#"{"type":"done"}"#;

/// A path on the device as the client sent it: UTF-8, not empty, no NUL, shorter than
/// [`MAX_PATH_BYTES`]. The hub passes it to SFTP as it is.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct DevicePath(String);

impl DevicePath {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Shows the length only, so a logged value holds no path (LOG-007).
impl std::fmt::Debug for DevicePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DevicePath({} bytes)", self.0.len())
    }
}

/// A client text message the hub accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Request {
    /// The entries of a directory.
    List(DevicePath),
    /// One file's bytes.
    Download(DevicePath),
    /// A new file of exactly `size` bytes, sent as binary frames after this message.
    Upload { path: DevicePath, size: u64 },
    /// Ends the transfer in flight; ignored when there is none.
    Cancel,
    /// The client is alive; the hub does not answer.
    Ping,
}

/// Why a client text message was refused. Holds nothing from the message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum BadRequest {
    #[error("the text message is longer than 4 KiB")]
    TooLong,
    #[error("the text message is not one of the files tunnel's messages")]
    Unknown,
    #[error("a path is empty or holds NUL")]
    BadPath,
}

/// The wire shape. `deny_unknown_fields` with struct variants refuses extra fields on every
/// message; serde refuses a missing or repeated field, a negative or fractional `size` and
/// one past `u64`. `parse` refuses the array form serde would also read.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Wire {
    List { path: String },
    Download { path: String },
    Upload { path: String, size: u64 },
    Cancel {},
    Ping {},
}

/// The message in `text`, or why it is refused.
///
/// # Errors
/// [`BadRequest::TooLong`] over [`MAX_TEXT_BYTES`], [`BadRequest::Unknown`] for anything that
/// is not exactly one K4 Section 9 client message, [`BadRequest::BadPath`] for a path outside
/// the rules above.
pub(super) fn parse(text: &str) -> Result<Request, BadRequest> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(BadRequest::TooLong);
    }
    if !text.trim_start_matches(JSON_SPACE).starts_with('{') {
        return Err(BadRequest::Unknown);
    }
    Ok(
        match serde_json::from_str::<Wire>(text).map_err(|_| BadRequest::Unknown)? {
            Wire::List { path } => Request::List(checked(path)?),
            Wire::Download { path } => Request::Download(checked(path)?),
            Wire::Upload { path, size } => Request::Upload {
                path: checked(path)?,
                size,
            },
            Wire::Cancel {} => Request::Cancel,
            Wire::Ping {} => Request::Ping,
        },
    )
}

/// `path` when it follows the path rules.
fn checked(path: String) -> Result<DevicePath, BadRequest> {
    let fits = !path.is_empty() && !path.contains('\0');
    fits.then_some(DevicePath(path)).ok_or(BadRequest::BadPath)
}

/// Why the hub refused a request (K4 Section 9 `refused.reason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum Refusal {
    /// The device's transfer policy does not allow downloads.
    DownloadOff,
    /// The device's transfer policy does not allow uploads.
    UploadOff,
    /// Nothing at that path, or not a regular file to download.
    NotFound,
    /// The device refused the request.
    Denied,
    /// Something is already at the upload's path; the hub never overwrites.
    Exists,
}

impl Refusal {
    /// The K4 name, as the audit entry records it.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::DownloadOff => "DOWNLOAD_OFF",
            Self::UploadOff => "UPLOAD_OFF",
            Self::NotFound => "NOT_FOUND",
            Self::Denied => "DENIED",
            Self::Exists => "EXISTS",
        }
    }
}

/// What a directory entry is; other kinds (sockets, pipes, devices) are not listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Kind {
    File,
    Dir,
    Link,
}

/// One directory entry as the client sees it. `size` and `modifiedAt` are `null` when the
/// device does not say.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) kind: Kind,
    pub(super) size: Option<u64>,
    pub(super) modified_at: Option<DateTime<Utc>>,
}

/// A directory's entries: `path` as the device resolved it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Listing {
    pub(super) path: String,
    pub(super) entries: Vec<Entry>,
}

/// The hub's messages that carry values.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Out<'a> {
    Entries { path: &'a str, entries: &'a [Entry] },
    Download { size: u64 },
    Refused { reason: Refusal },
}

fn render(message: &Out<'_>) -> String {
    serde_json::to_string(message).expect("the hub's messages always serialise")
}

/// `entries` for `listing`.
pub(super) fn entries(listing: &Listing) -> String {
    render(&Out::Entries {
        path: &listing.path,
        entries: &listing.entries,
    })
}

/// `download` announcing `size` bytes of binary frames.
pub(super) fn download(size: u64) -> String {
    render(&Out::Download { size })
}

/// `refused` with `reason`.
pub(super) fn refused(reason: Refusal) -> String {
    render(&Out::Refused { reason })
}
