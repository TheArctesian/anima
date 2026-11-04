//! Keyset paging of sessions, newest first by `(started_at, id)` (API-006).
//!
//! Pure. The cursor is opaque to the client: a version prefix and base64url of the start time
//! in microseconds and the session id, which together are unique and fixed for a row.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::model::{FieldError, Session};
use crate::ids::SessionId;

const CURSOR_PREFIX: &str = "s1.";
const CURSOR_BYTES: usize = 8 + 16;
const MAX_PAGE_SIZE: usize = 200;

/// The row a page starts after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cursor {
    pub(super) started_at: DateTime<Utc>,
    pub(super) id: SessionId,
}

impl Cursor {
    pub(super) const fn of(session: &Session) -> Self {
        Self {
            started_at: session.started_at,
            id: session.id,
        }
    }
}

/// One page of rows and whether more follow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Page<T> {
    pub(super) rows: Vec<T>,
    pub(super) has_next_page: bool,
}

/// The page size the operator asked for (`first`), 1 to 200.
///
/// # Errors
/// A `first` error when it is outside that range.
pub(super) fn page_size(first: i32) -> Result<usize, FieldError> {
    match usize::try_from(first) {
        Ok(size) if (1..=MAX_PAGE_SIZE).contains(&size) => Ok(size),
        _ => Err(FieldError::new(
            "first",
            format!("Ask for between 1 and {MAX_PAGE_SIZE} sessions per page."),
        )),
    }
}

#[must_use]
pub(super) fn encode_cursor(cursor: Cursor) -> String {
    let mut bytes = [0_u8; CURSOR_BYTES];
    bytes[..8].copy_from_slice(&cursor.started_at.timestamp_micros().to_be_bytes());
    bytes[8..].copy_from_slice(Uuid::from(cursor.id).as_bytes());
    format!("{CURSOR_PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes))
}

/// The row a cursor points after.
///
/// # Errors
/// An `after` error when the text is not a cursor this module made.
pub(super) fn decode_cursor(text: &str) -> Result<Cursor, FieldError> {
    let bad = || {
        FieldError::new(
            "after",
            "That page cursor is not valid. Start again from the first page.",
        )
    };
    let encoded = text
        .strip_prefix(CURSOR_PREFIX)
        .filter(|rest| rest.len() <= CURSOR_BYTES.div_ceil(3) * 4)
        .ok_or_else(bad)?;
    let bytes: [u8; CURSOR_BYTES] = URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(bad)?;
    let (micros, id) = bytes.split_at(8);
    let micros = i64::from_be_bytes(micros.try_into().map_err(|_| bad())?);
    let started_at = DateTime::from_timestamp_micros(micros).ok_or_else(bad)?;
    let id = Uuid::from_slice(id).map_err(|_| bad())?;
    Ok(Cursor {
        started_at,
        id: SessionId::from(id),
    })
}

/// Turns rows fetched with limit `size + 1` into a page of at most `size` rows.
#[must_use]
pub(super) fn split_page<T>(mut fetched: Vec<T>, size: usize) -> Page<T> {
    let has_next_page = fetched.len() > size;
    fetched.truncate(size);
    Page {
        rows: fetched,
        has_next_page,
    }
}
