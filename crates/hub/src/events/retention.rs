//! How long each log keeps its entries (DB-005, LOG-006), as the first UTC day to keep.

use chrono::{DateTime, Days, Months, NaiveDate, Utc};

/// A partitioned log. Its retention matches `events.drop_day_partitions_before`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Log {
    /// `event_log`: 90 days.
    Events,
    /// `audit_log`: 1 year.
    Audit,
}

pub(super) const LOGS: [Log; 2] = [Log::Events, Log::Audit];

impl Log {
    /// The table name the partition functions accept.
    pub(super) const fn table(self) -> &'static str {
        match self {
            Self::Events => "event_log",
            Self::Audit => "audit_log",
        }
    }

    /// Partitions for UTC days before this one may be dropped at `now`. It is one day later
    /// than the database allows, so a hub clock slightly ahead of the database clock is never
    /// refused; the extra day is dropped on the next run.
    pub(super) fn first_kept_day(self, now: DateTime<Utc>) -> NaiveDate {
        let now = now.naive_utc();
        let bound = match self {
            Self::Events => now.checked_sub_days(Days::new(90)),
            Self::Audit => now.checked_sub_months(Months::new(12)),
        };
        bound
            .and_then(|bound| bound.date().pred_opt())
            .unwrap_or(NaiveDate::MIN)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use super::Log;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn db_005_logs_keep_ninety_days_and_one_year_plus_a_day_of_clock_margin() {
        let now = Utc.with_ymd_and_hms(2028, 2, 29, 0, 30, 0).unwrap();
        assert_eq!(Log::Events.first_kept_day(now), date(2027, 11, 30));
        assert_eq!(Log::Audit.first_kept_day(now), date(2027, 2, 27));
    }
}
