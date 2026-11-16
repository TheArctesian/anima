//! The opaque cursor of a timeline page (API-006). Pure.

use chrono::DateTime;

use super::model::Mark;
use crate::engine::records::Reference;

const PREFIX: &str = "t1.";

/// The cursor that points just after `mark`: the prefix, the start in microseconds since the
/// epoch, a dot and the reference.
pub(super) fn encode(mark: &Mark) -> String {
    format!(
        "{PREFIX}{}.{}",
        mark.start.timestamp_micros(),
        mark.reference
    )
}

/// The mark a cursor points after; `None` for any text [`encode`] did not make.
pub(super) fn decode(cursor: &str) -> Option<Mark> {
    let (micros, reference) = cursor.strip_prefix(PREFIX)?.split_once('.')?;
    let mark = Mark {
        start: DateTime::from_timestamp_micros(micros.parse().ok()?)?,
        reference: Reference::parse(reference).ok()?,
    };
    (encode(&mark) == cursor).then_some(mark)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone as _, Utc};

    use super::*;

    #[test]
    fn api_006_a_cursor_round_trips_and_only_its_exact_text_is_accepted() {
        let mark = Mark {
            start: Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
                + TimeDelta::microseconds(123_456),
            reference: Reference::parse("book:isbn.978").unwrap(),
        };
        let cursor = encode(&mark);
        assert_eq!(cursor, "t1.1904212800123456.book:isbn.978");
        assert_eq!(decode(&cursor), Some(mark));
        for bad in [
            "",
            "t1.",
            "t1.1904212800123456",
            "t1.+1904212800123456.book:1",
            "t1.x.book:1",
            "t1.1904212800123456.Book:1",
            "e1.1904212800123456.book:1",
        ] {
            assert_eq!(decode(bad), None, "{bad:?}");
        }
    }
}
