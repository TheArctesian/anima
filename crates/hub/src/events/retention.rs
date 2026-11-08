//! How long each log keeps its entries (DB-005, LOG-006), as the first UTC day to keep.

use chrono::{DateTime, Days, Months, NaiveDate, Utc};

/// A partitioned log. Its floor is in `events.drop_day_partitions_before`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Log {
    /// `event_log`: the period the operator set (D-25), at least 30 days.
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

    /// Partitions for UTC days before this one may be dropped at `now`, when `event_log` keeps
    /// `events_keep`. It is one day later than the database allows, so a hub clock slightly
    /// ahead of the database clock is never refused; the extra day is dropped on the next run.
    pub(super) fn first_kept_day(self, now: DateTime<Utc>, events_keep: Days) -> NaiveDate {
        let now = now.naive_utc();
        let bound = match self {
            Self::Events => now.checked_sub_days(events_keep),
            Self::Audit => now.checked_sub_months(Months::new(12)),
        };
        bound
            .and_then(|bound| bound.date().pred_opt())
            .unwrap_or(NaiveDate::MIN)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Days, NaiveDate, TimeZone, Utc};

    use super::Log;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn db_005_the_event_log_keeps_its_period_and_the_audit_log_one_year_plus_a_day_of_margin() {
        let now = Utc.with_ymd_and_hms(2028, 2, 29, 0, 30, 0).unwrap();
        assert_eq!(
            Log::Events.first_kept_day(now, Days::new(90)),
            date(2027, 11, 30)
        );
        assert_eq!(
            Log::Events.first_kept_day(now, Days::new(30)),
            date(2028, 1, 29)
        );
        assert_eq!(
            Log::Audit.first_kept_day(now, Days::new(30)),
            date(2027, 2, 27)
        );
        assert_eq!(
            Log::Audit.first_kept_day(now, Days::new(365)),
            date(2027, 2, 27)
        );
    }
}
