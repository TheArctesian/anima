//! When a window is active: pure rule evaluation (MW-001, MW-002, MW-004).
//!
//! Nothing here reads a clock or the database; the caller passes `now`.
//!
//! Rules for local times:
//! - An occurrence starts on a local calendar day at a local time in the window's time zone.
//! - If that local time does not exist (a spring-forward gap), the occurrence starts at the
//!   first instant after the gap.
//! - If it exists twice (a fall-back repeat), the occurrence starts at the earlier instant.
//! - An occurrence lasts `duration_minutes` of elapsed time, so on a daylight-saving change
//!   its wall-clock end moves by the change.
//! - An occurrence may end after midnight, or days later: every day that could still hold a
//!   running occurrence is checked.

use chrono::{
    DateTime, Datelike, Days, NaiveDate, NaiveDateTime, TimeDelta, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;

use super::model::{Weekday, WindowRule};

const MINUTES_PER_DAY: u32 = 24 * 60;
/// The longest spring-forward gap the search steps over (two days; Samoa once skipped one).
const GAP_SEARCH_MINUTES: u32 = 2 * MINUTES_PER_DAY;

/// Whether the rule is in force at `now` for a window in time zone `tz`.
///
/// A manual rule is never active here: the on/off state of a manual window is stored with the
/// window, so use `Window::is_active` for one.
#[must_use]
pub fn is_active(rule: &WindowRule, tz: Tz, now: DateTime<Utc>) -> bool {
    match rule {
        WindowRule::Manual => false,
        WindowRule::Single { starts_at, ends_at } => *starts_at <= now && now < *ends_at,
        _ => rule
            .duration_minutes()
            .is_some_and(|minutes| recurring_is_active(rule, tz, now, minutes)),
    }
}

/// Checks the local day of `now` and every earlier day an occurrence could still run from: as
/// many days as the duration spans, plus one because a day with a clock change is shorter.
fn recurring_is_active(rule: &WindowRule, tz: Tz, now: DateTime<Utc>, minutes: u32) -> bool {
    let length = TimeDelta::minutes(i64::from(minutes));
    let today = now.with_timezone(&tz).date_naive();
    let days_back = u64::from(minutes.div_ceil(MINUTES_PER_DAY)) + 1;
    (0..=days_back)
        .filter_map(|back| today.checked_sub_days(Days::new(back)))
        .filter_map(|date| occurrence_start(rule, tz, date))
        .any(|start| {
            start <= now
                && start
                    .checked_add_signed(length)
                    .is_some_and(|end| now < end)
        })
}

/// The instant the occurrence that starts on local `date` begins, if the rule has one then.
fn occurrence_start(rule: &WindowRule, tz: Tz, date: NaiveDate) -> Option<DateTime<Utc>> {
    match rule {
        WindowRule::RecurringWeekday {
            weekdays,
            start_time,
            ..
        } => weekdays
            .contains(&Weekday::from(date.weekday()))
            .then(|| local_instant(tz, date.and_time(*start_time))),
        WindowRule::RecurringDayOfMonth {
            days_of_month,
            start_time,
            ..
        } => on_day_of_month(days_of_month, date)
            .then(|| local_instant(tz, date.and_time(*start_time))),
        WindowRule::RecurringInterval {
            first_start_at,
            every_days,
            ..
        } => {
            let first = first_start_at.with_timezone(&tz).naive_local();
            let elapsed = date.signed_duration_since(first.date()).num_days();
            let due = elapsed >= 0 && elapsed.checked_rem(i64::from(*every_days)) == Some(0);
            match (due, elapsed) {
                (false, _) => None,
                // The first occurrence is exactly the instant the operator gave, even when
                // that local time is the later of two.
                (true, 0) => Some(*first_start_at),
                (true, _) => Some(local_instant(tz, date.and_time(first.time()))),
            }
        }
        WindowRule::Manual | WindowRule::Single { .. } => None,
    }
}

/// Whether `date` is one of the listed days; `-1` is the last day of its month.
fn on_day_of_month(days: &[i8], date: NaiveDate) -> bool {
    let is_last = date
        .succ_opt()
        .is_none_or(|next| next.month() != date.month());
    days.iter().any(|day| {
        if *day == -1 {
            is_last
        } else {
            u32::try_from(*day).is_ok_and(|day| day == date.day())
        }
    })
}

/// The instant of a local time: the earlier one when it repeats, the end of the gap when it
/// does not exist.
fn local_instant(tz: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    tz.from_local_datetime(&local)
        .earliest()
        .map_or_else(|| after_gap(tz, local), |instant| instant.to_utc())
}

/// The first instant after the gap that holds `local`. Gaps begin on a whole minute, so
/// stepping by minutes lands exactly on the change. If no end is found, `local` is read as UTC.
fn after_gap(tz: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    let mut probe = local
        .with_second(0)
        .and_then(|time| time.with_nanosecond(0))
        .unwrap_or(local);
    for _ in 0..GAP_SEARCH_MINUTES {
        let Some(next) = probe.checked_add_signed(TimeDelta::minutes(1)) else {
            break;
        };
        probe = next;
        if let Some(instant) = tz.from_local_datetime(&probe).earliest() {
            return instant.to_utc();
        }
    }
    Utc.from_utc_datetime(&local)
}
