//! Validation of a window typed in the form (MW-001, MW-004).
//!
//! Every function is pure. The validator reports all field errors at once so the form can show
//! them together.

use std::{collections::HashSet, hash::Hash, str::FromStr};

use chrono::NaiveTime;
use chrono_tz::Tz;

use super::{
    model::{NewWindow, Weekday, WindowForm, WindowKind, WindowRule},
    targets,
};
use crate::text::is_plain;

pub use crate::api_errors::FieldError;

const NAME_MAX: usize = 100;
/// Seven days; the database constraint on a stored rule says the same.
pub(super) const DURATION_MAX_MINUTES: u32 = 7 * 24 * 60;
/// Days 1 to 31 and -1: no list of distinct days is longer. Lengths are checked before any
/// scan of the list, so an oversized input costs nothing (API-005).
const DAYS_OF_MONTH_MAX: usize = 32;
const WEEKDAYS_MAX: usize = 7;

/// Collects field errors while the remaining fields are still checked.
#[derive(Default)]
pub(super) struct Problems(pub(super) Vec<FieldError>);

impl Problems {
    pub(super) fn field<T>(&mut self, checked: Result<T, FieldError>) -> Option<T> {
        checked.map_err(|error| self.0.push(error)).ok()
    }

    pub(super) fn fields<T>(&mut self, checked: Result<T, Vec<FieldError>>) -> Option<T> {
        checked.map_err(|errors| self.0.extend(errors)).ok()
    }

    pub(super) fn finish<T>(self, value: Option<T>) -> Result<T, Vec<FieldError>> {
        match value {
            Some(value) if self.0.is_empty() => Ok(value),
            _ => Err(self.0),
        }
    }
}

/// Checks a window to create or to replace one with.
///
/// # Errors
/// Every invalid field, all at once.
pub(super) fn validate(form: &WindowForm) -> Result<NewWindow, Vec<FieldError>> {
    let mut problems = Problems::default();
    let name = problems.field(name(&form.name));
    let timezone = problems.field(timezone(&form.timezone));
    let rule = problems.fields(rule(form));
    let targets = problems.fields(targets::validate(&form.targets));
    let checked = match (name, timezone, rule, targets) {
        (Some(name), Some(timezone), Some(rule), Some(targets)) => Some(NewWindow {
            name,
            timezone,
            rule,
            targets,
        }),
        _ => None,
    };
    problems.finish(checked)
}

fn name(text: &str) -> Result<String, FieldError> {
    let name = text.trim();
    if !is_plain(name) {
        Err(FieldError::new(
            "name",
            "The name cannot contain control characters or text-direction marks.",
        ))
    } else if (1..=NAME_MAX).contains(&name.chars().count()) {
        Ok(name.to_owned())
    } else {
        Err(FieldError::new(
            "name",
            format!("The name must be 1 to {NAME_MAX} characters."),
        ))
    }
}

/// An IANA time zone name such as `Pacific/Auckland` (MW-001).
fn timezone(text: &str) -> Result<Tz, FieldError> {
    Tz::from_str(text.trim()).map_err(|_| {
        FieldError::new(
            "timezone",
            "Use an IANA time zone name such as Pacific/Auckland or UTC.",
        )
    })
}

/// The rule fields each kind uses; every other rule field must be absent.
const fn used_fields(kind: WindowKind) -> &'static [&'static str] {
    match kind {
        WindowKind::Manual => &[],
        WindowKind::Single => &["startsAt", "endsAt"],
        WindowKind::RecurringInterval => &["firstStartAt", "everyDays", "durationMinutes"],
        WindowKind::RecurringWeekday => &["weekdays", "startTime", "durationMinutes"],
        WindowKind::RecurringDayOfMonth => &["daysOfMonth", "startTime", "durationMinutes"],
    }
}

/// Which rule fields the form filled in.
fn filled_fields(form: &WindowForm) -> [(&'static str, bool); 8] {
    [
        ("startsAt", form.starts_at.is_some()),
        ("endsAt", form.ends_at.is_some()),
        ("firstStartAt", form.first_start_at.is_some()),
        ("everyDays", form.every_days.is_some()),
        ("weekdays", form.weekdays.is_some()),
        ("daysOfMonth", form.days_of_month.is_some()),
        ("startTime", form.start_time.is_some()),
        ("durationMinutes", form.duration_minutes.is_some()),
    ]
}

/// A field the kind needs but lacks, or a field another kind owns.
fn field_mismatches(form: &WindowForm) -> Vec<FieldError> {
    let used = used_fields(form.kind);
    let kind = form.kind.as_str();
    let mut errors = Vec::new();
    for (field, filled) in filled_fields(form) {
        match (used.contains(&field), filled) {
            (true, false) => errors.push(FieldError::new(
                field,
                format!("A {kind} window needs this field."),
            )),
            (false, true) => errors.push(FieldError::new(
                field,
                format!("A {kind} window does not use this field."),
            )),
            _ => {}
        }
    }
    errors
}

fn rule(form: &WindowForm) -> Result<WindowRule, Vec<FieldError>> {
    let mut problems = Problems(field_mismatches(form));
    let rule = match form.kind {
        WindowKind::Manual => Some(WindowRule::Manual),
        WindowKind::Single => single(form, &mut problems),
        WindowKind::RecurringInterval => interval(form, &mut problems),
        WindowKind::RecurringWeekday => weekday(form, &mut problems),
        WindowKind::RecurringDayOfMonth => day_of_month(form, &mut problems),
    };
    problems.finish(rule)
}

fn single(form: &WindowForm, problems: &mut Problems) -> Option<WindowRule> {
    let (starts_at, ends_at) = form.starts_at.zip(form.ends_at)?;
    if ends_at <= starts_at {
        problems.0.push(FieldError::new(
            "endsAt",
            "The window must end after it starts.",
        ));
        return None;
    }
    Some(WindowRule::Single { starts_at, ends_at })
}

fn interval(form: &WindowForm, problems: &mut Problems) -> Option<WindowRule> {
    let every_days = form
        .every_days
        .and_then(|days| problems.field(every_days(days)));
    let duration_minutes = form
        .duration_minutes
        .and_then(|minutes| problems.field(duration(minutes)));
    Some(WindowRule::RecurringInterval {
        first_start_at: form.first_start_at?,
        every_days: every_days?,
        duration_minutes: duration_minutes?,
    })
}

fn weekday(form: &WindowForm, problems: &mut Problems) -> Option<WindowRule> {
    let weekdays = form
        .weekdays
        .as_deref()
        .and_then(|days| problems.field(weekdays(days)));
    let start_time = form
        .start_time
        .as_deref()
        .and_then(|time| problems.field(start_time(time)));
    let duration_minutes = form
        .duration_minutes
        .and_then(|minutes| problems.field(duration(minutes)));
    Some(WindowRule::RecurringWeekday {
        weekdays: weekdays?,
        start_time: start_time?,
        duration_minutes: duration_minutes?,
    })
}

fn day_of_month(form: &WindowForm, problems: &mut Problems) -> Option<WindowRule> {
    let days_of_month = form
        .days_of_month
        .as_deref()
        .and_then(|days| problems.field(days_of_month(days)));
    let start_time = form
        .start_time
        .as_deref()
        .and_then(|time| problems.field(start_time(time)));
    let duration_minutes = form
        .duration_minutes
        .and_then(|minutes| problems.field(duration(minutes)));
    Some(WindowRule::RecurringDayOfMonth {
        days_of_month: days_of_month?,
        start_time: start_time?,
        duration_minutes: duration_minutes?,
    })
}

fn every_days(days: i32) -> Result<u32, FieldError> {
    u32::try_from(days)
        .ok()
        .filter(|days| *days >= 1)
        .ok_or_else(|| FieldError::new("everyDays", "Repeat every 1 day or more."))
}

fn duration(minutes: i32) -> Result<u32, FieldError> {
    u32::try_from(minutes)
        .ok()
        .filter(|minutes| (1..=DURATION_MAX_MINUTES).contains(minutes))
        .ok_or_else(|| {
            FieldError::new(
                "durationMinutes",
                format!("The duration must be 1 to {DURATION_MAX_MINUTES} minutes (7 days)."),
            )
        })
}

/// `HH:MM`, 24-hour, with two digits each.
fn start_time(text: &str) -> Result<NaiveTime, FieldError> {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 5
        && bytes[2] == b':'
        && bytes[..2].iter().chain(&bytes[3..]).all(u8::is_ascii_digit);
    shaped
        .then(|| NaiveTime::parse_from_str(text, "%H:%M").ok())
        .flatten()
        .ok_or_else(|| FieldError::new("startTime", "Use the 24-hour form HH:MM, such as 03:00."))
}

fn weekdays(days: &[Weekday]) -> Result<Vec<Weekday>, FieldError> {
    if days.is_empty() {
        Err(FieldError::new("weekdays", "Choose at least one weekday."))
    } else if days.len() > WEEKDAYS_MAX {
        Err(FieldError::new(
            "weekdays",
            format!("A week has {WEEKDAYS_MAX} days."),
        ))
    } else if has_repeat(days) {
        Err(FieldError::new("weekdays", "Each weekday can appear once."))
    } else {
        Ok(days.to_vec())
    }
}

fn days_of_month(days: &[i32]) -> Result<Vec<i8>, FieldError> {
    let message = "Use days 1 to 31, or -1 for the last day of the month.";
    if days.is_empty() {
        return Err(FieldError::new("daysOfMonth", "Choose at least one day."));
    }
    if days.len() > DAYS_OF_MONTH_MAX {
        return Err(FieldError::new("daysOfMonth", message));
    }
    if has_repeat(days) {
        return Err(FieldError::new("daysOfMonth", "Each day can appear once."));
    }
    days.iter()
        .map(|day| {
            i8::try_from(*day)
                .ok()
                .filter(|day| *day == -1 || (1..=31).contains(day))
                .ok_or_else(|| FieldError::new("daysOfMonth", message))
        })
        .collect()
}

fn has_repeat<T: Eq + Hash>(values: &[T]) -> bool {
    let mut seen = HashSet::with_capacity(values.len());
    values.iter().any(|value| !seen.insert(value))
}
