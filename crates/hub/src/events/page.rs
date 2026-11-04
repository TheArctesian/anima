//! Keyset paging over a log, newest first (API-006, R-014): page sizes and the opaque cursors.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::retention::Log;

/// The most entries one page may hold.
pub(super) const MAX_PAGE_SIZE: usize = 200;

/// An entry's place in newest-first order: by `at`, then by `id`, both descending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Position {
    pub(super) at: DateTime<Utc>,
    pub(super) id: Uuid,
}

/// One page of entries and whether more follow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Page<T> {
    pub(super) rows: Vec<T>,
    pub(super) has_next_page: bool,
}

/// The page size the operator asked for (`first`), if it is between 1 and [`MAX_PAGE_SIZE`].
pub(super) fn page_size(first: i32) -> Option<usize> {
    usize::try_from(first)
        .ok()
        .filter(|size| (1..=MAX_PAGE_SIZE).contains(size))
}

/// How many rows to fetch for a page of `size`: one more, to learn whether a next page exists.
pub(super) fn fetch_limit(size: usize) -> i64 {
    i64::try_from(size.saturating_add(1)).unwrap_or(i64::MAX)
}

/// Turns rows fetched with [`fetch_limit`] into a page of at most `size` rows.
pub(super) fn split_page<T>(mut fetched: Vec<T>, size: usize) -> Page<T> {
    let has_next_page = fetched.len() > size;
    fetched.truncate(size);
    Page {
        rows: fetched,
        has_next_page,
    }
}

/// Each log has its own prefix, so a cursor from one list is refused by the other.
const fn prefix(log: Log) -> &'static str {
    match log {
        Log::Events => "e1.",
        Log::Audit => "a1.",
    }
}

/// The opaque cursor that points just after `position` in `log`: the prefix, the instant in
/// microseconds since the epoch, a dot, and the id as 32 hex digits.
pub(super) fn encode_cursor(log: Log, position: Position) -> String {
    format!(
        "{}{}.{}",
        prefix(log),
        position.at.timestamp_micros(),
        position.id.simple()
    )
}

/// The position a cursor of `log` points after; `None` for any text [`encode_cursor`] did not
/// make for that log. Only the exact text it makes is accepted, so no two cursors name one
/// position.
pub(super) fn decode_cursor(log: Log, cursor: &str) -> Option<Position> {
    let (micros, id) = cursor.strip_prefix(prefix(log))?.split_once('.')?;
    let at = DateTime::from_timestamp_micros(micros.parse().ok()?)?;
    let id = Uuid::try_parse(id).ok()?;
    let position = Position { at, id };
    (encode_cursor(log, position) == cursor).then_some(position)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    fn position() -> Position {
        Position {
            at: Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
                + chrono::TimeDelta::microseconds(123_456),
            id: "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d".parse().unwrap(),
        }
    }

    #[test]
    fn api_006_a_cursor_round_trips_to_its_position() {
        for log in [Log::Events, Log::Audit] {
            let cursor = encode_cursor(log, position());
            assert_eq!(decode_cursor(log, &cursor), Some(position()), "{cursor}");
        }
        assert_eq!(
            encode_cursor(Log::Events, position()),
            "e1.1904212800123456.0192f5a07c3e7a529d1b3f4e5a6b7c8d"
        );
    }

    #[test]
    fn api_006_a_cursor_of_one_log_is_refused_by_the_other() {
        let events = encode_cursor(Log::Events, position());
        let audit = encode_cursor(Log::Audit, position());
        assert_eq!(decode_cursor(Log::Audit, &events), None);
        assert_eq!(decode_cursor(Log::Events, &audit), None);
    }

    #[test]
    fn api_006_only_the_exact_cursor_text_is_accepted() {
        let good = encode_cursor(Log::Events, position());
        let hyphenated = format!("e1.1904212800123456.{}", position().id.hyphenated());
        for bad in [
            String::new(),
            "e1.".to_owned(),
            "e1.1904212800123456".to_owned(),
            "e1.+1904212800123456.0192f5a07c3e7a529d1b3f4e5a6b7c8d".to_owned(),
            "e1.01904212800123456.0192f5a07c3e7a529d1b3f4e5a6b7c8d".to_owned(),
            "e1.1904212800123456.0192F5A07C3E7A529D1B3F4E5A6B7C8D".to_owned(),
            "e1.x.0192f5a07c3e7a529d1b3f4e5a6b7c8d".to_owned(),
            "e1.1904212800123456.not-a-uuid".to_owned(),
            "e1.9223372036854775807.0192f5a07c3e7a529d1b3f4e5a6b7c8d".to_owned(),
            hyphenated,
            format!("{good} "),
            format!("x{good}"),
        ] {
            assert_eq!(decode_cursor(Log::Events, &bad), None, "{bad:?}");
        }
    }

    #[test]
    fn api_006_page_size_is_between_one_and_the_maximum() {
        assert_eq!(page_size(1), Some(1));
        assert_eq!(page_size(200), Some(MAX_PAGE_SIZE));
        for refused in [0, -1, 201, i32::MIN, i32::MAX] {
            assert_eq!(page_size(refused), None, "{refused}");
        }
    }

    #[test]
    fn api_006_a_page_keeps_size_rows_and_tells_whether_more_follow() {
        assert_eq!(fetch_limit(2), 3);
        assert_eq!(fetch_limit(usize::MAX), i64::MAX);
        let full = split_page(vec![1, 2, 3], 2);
        assert_eq!((full.rows, full.has_next_page), (vec![1, 2], true));
        let last = split_page(vec![1, 2], 2);
        assert_eq!((last.rows, last.has_next_page), (vec![1, 2], false));
    }
}
