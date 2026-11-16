//! The one form of an event instant that Anima machines exchange: microseconds since the Unix
//! epoch in UTC (K-E3 Section 2; TML-008). A time zone changes how an instant is shown, never
//! the instant. Pure.

use chrono::{DateTime, Utc};

/// The instant as microseconds since 1970-01-01T00:00:00Z. Precision below a microsecond is
/// cut, as PostgreSQL stores it.
#[must_use]
pub fn micros(at: DateTime<Utc>) -> i64 {
    at.timestamp_micros()
}

/// The instant `micros` microseconds after the epoch; `None` outside the range chrono holds.
#[must_use]
pub fn from_micros(micros: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_micros(micros)
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone as _};

    use super::*;

    fn read(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn tml_008_one_instant_written_in_three_time_zones_is_one_count() {
        let written = [
            "2026-11-16T12:00:00.123456Z",
            "2026-11-16T20:00:00.123456+08:00",
            "2026-11-16T07:00:00.123456-05:00",
        ];
        let counts: Vec<i64> = written.iter().map(|text| micros(read(text))).collect();
        assert_eq!(counts, [1_794_830_400_123_456; 3]);
        assert_eq!(from_micros(counts[0]), Some(read(written[0])));
    }

    #[test]
    fn tml_008_showing_an_instant_in_a_zone_does_not_change_it() {
        let at = read("2026-11-16T12:00:00Z");
        let shown = at.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
        assert_eq!(shown.to_rfc3339(), "2026-11-16T21:00:00+09:00");
        assert_eq!(micros(shown.with_timezone(&Utc)), micros(at));
    }

    #[test]
    fn tml_008_precision_below_a_microsecond_is_cut_and_the_range_is_checked() {
        let fine = Utc.timestamp_opt(1_794_830_400, 123_456_789).unwrap();
        assert_eq!(micros(fine), 1_794_830_400_123_456);
        assert_eq!(from_micros(i64::MAX), None);
        assert_eq!(from_micros(0), Some(read("1970-01-01T00:00:00Z")));
    }
}
