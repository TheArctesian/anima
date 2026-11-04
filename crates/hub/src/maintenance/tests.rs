//! Unit tests of the pure rules. Behaviour against the database and GraphQL is tested in
//! `tests/it/maintenance`.

mod coverage;
mod daylight_saving;
mod kinds;
mod month_end;
mod validation;

use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;

use super::{
    model::{Weekday, WindowRule},
    schedule::is_active,
};

const AUCKLAND: Tz = Tz::Pacific__Auckland;
const NEW_YORK: Tz = Tz::America__New_York;
const UTC: Tz = Tz::UTC;

fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

fn time(text: &str) -> NaiveTime {
    NaiveTime::parse_from_str(text, "%H:%M").unwrap()
}

fn weekly(days: &[Weekday], start: &str, minutes: u32) -> WindowRule {
    WindowRule::RecurringWeekday {
        weekdays: days.to_vec(),
        start_time: time(start),
        duration_minutes: minutes,
    }
}

fn monthly(days: &[i8], start: &str, minutes: u32) -> WindowRule {
    WindowRule::RecurringDayOfMonth {
        days_of_month: days.to_vec(),
        start_time: time(start),
        duration_minutes: minutes,
    }
}

/// Whether the rule is active at each instant, in the order given.
fn states(rule: &WindowRule, zone: Tz, instants: &[&str]) -> Vec<bool> {
    instants
        .iter()
        .map(|instant| is_active(rule, zone, at(instant)))
        .collect()
}
