//! Keyset paging over releases, newest first (API-006, R-014): the page size and the opaque
//! cursor.
//!
//! Pure.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::model::{FieldError, Release};
use crate::ids::ReleaseId;

/// The most releases one page may hold.
const MAX_PAGE_SIZE: usize = 100;

/// The cursor prefix, so a cursor of another list is refused.
const PREFIX: &str = "r1.";

/// One page of releases and whether more follow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleasePage {
    pub releases: Vec<Release>,
    pub has_next_page: bool,
}

/// The page size the operator asked for (`first`), between 1 and 100.
pub(super) fn page_size(first: i32) -> Result<usize, FieldError> {
    usize::try_from(first)
        .ok()
        .filter(|size| (1..=MAX_PAGE_SIZE).contains(size))
        .ok_or_else(|| FieldError::new("first", "Ask for 1 to 100 releases."))
}

/// The opaque cursor just after this release: the prefix, its creation instant in
/// microseconds, a dot, and its id as 32 hex digits.
#[must_use]
pub fn cursor(release: &Release) -> String {
    format!(
        "{PREFIX}{}.{}",
        release.created_at.timestamp_micros(),
        Uuid::from(release.id).simple()
    )
}

/// The position a cursor points after. Only the exact text [`cursor`] makes is accepted.
pub(super) fn position(text: &str) -> Result<(DateTime<Utc>, ReleaseId), FieldError> {
    let refused = || FieldError::new("after", "That cursor is not valid.");
    let (micros, id) = text
        .strip_prefix(PREFIX)
        .and_then(|rest| rest.split_once('.'))
        .ok_or_else(refused)?;
    let at = micros
        .parse()
        .ok()
        .and_then(DateTime::from_timestamp_micros)
        .ok_or_else(refused)?;
    let id = Uuid::try_parse(id).map_err(|_| refused())?;
    let canonical = format!("{PREFIX}{}.{}", at.timestamp_micros(), id.simple());
    if canonical != text {
        return Err(refused());
    }
    Ok((at, ReleaseId::from(id)))
}

/// Turns releases fetched with one more than `size` into a page.
pub(super) fn split(mut fetched: Vec<Release>, size: usize) -> ReleasePage {
    let has_next_page = fetched.len() > size;
    fetched.truncate(size);
    ReleasePage {
        releases: fetched,
        has_next_page,
    }
}
