//! The instants of a time rule (K-E6 Section 2). Pure: the caller gives `now`.
//!
//! A local time that a change of the clocks skips is due at the next minute that exists; a
//! local time that happens two times is due one time, at the earlier instant.

use chrono::{DateTime, Datelike as _, Days, NaiveDate, NaiveTime, TimeDelta, TimeZone as _, Utc};
use chrono_tz::Tz;

use super::model::TimeTrigger;

/// The days that the search looks back: one week of a rule with one weekday, and one more day.
const DAYS_BACK: u64 = 8;
/// The longest gap of a change of the clocks that the search steps over, in minutes.
const GAP_SEARCH_MINUTES: i64 = 2 * 24 * 60;

/// The instant of `at` on `date` in `zone`.
fn instant(zone: Tz, date: NaiveDate, at: NaiveTime) -> Option<DateTime<Utc>> {
    let local = date.and_time(at);
    (0..=GAP_SEARCH_MINUTES)
        .filter_map(|minutes| local.checked_add_signed(TimeDelta::minutes(minutes)))
        .find_map(|local| zone.from_local_datetime(&local).earliest())
        .map(|found| found.with_timezone(&Utc))
}

/// The latest instant of `trigger` that is not after `now`; `None` when there is none in the
/// last 8 days. A hub that was stopped thus fires a rule that it missed one time.
pub(super) fn latest(trigger: &TimeTrigger, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match trigger {
        TimeTrigger::Once(at) => (*at <= now).then_some(*at),
        TimeTrigger::Daily { at, days, zone } => {
            let today = now.with_timezone(zone).date_naive();
            (0..=DAYS_BACK)
                .filter_map(|back| today.checked_sub_days(Days::new(back)))
                .filter(|date| days.is_empty() || days.contains(&date.weekday()))
                .filter_map(|date| instant(*zone, date, *at))
                .find(|found| *found <= now)
        }
    }
}

/// The start of the day of `now` in `zone`, as an instant.
pub(super) fn day_start(zone: Tz, now: DateTime<Utc>) -> DateTime<Utc> {
    let today = now.with_timezone(&zone).date_naive();
    instant(zone, today, NaiveTime::MIN).unwrap_or(now)
}
