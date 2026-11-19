//! Reads the stored form of a trigger and of `unless`, and checks each field. Pure.

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use serde_json::Value;

use super::{Fault, On, StoredTrigger, StoredUnless, TIME_FORM, day};
use crate::engine::{
    records::Reference,
    reminders::model::{Crossing, TimeTrigger, Trigger, Unless},
};

const fn fault(field: &'static str, message: &'static str) -> Fault {
    Fault { field, message }
}

/// The states of a source account that a rule can wait for (K-E1 Section 9).
const STATES: [&str; 3] = ["connected", "stalled", "needs_reconnect"];
/// Lower-case letters, digits and underscores, starting with a letter, at most 40 characters:
/// the form of a reference kind.
fn is_word(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && text.len() <= 40
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// `<extension>.<name>`, each a word: the form of a measure key and of an expiry subject.
fn is_dotted(text: &str) -> bool {
    text.split_once('.')
        .is_some_and(|(extension, name)| is_word(extension) && is_word(name))
}

fn account_reference(id: uuid::Uuid) -> Option<Reference> {
    Reference::parse(&format!("source_account:{id}")).ok()
}

fn time(
    at: Option<&str>,
    days: &[String],
    zone: Option<&str>,
    once: Option<DateTime<Utc>>,
) -> Result<TimeTrigger, Fault> {
    if let Some(once) = once {
        return if at.is_none() && days.is_empty() && zone.is_none() {
            Ok(TimeTrigger::Once(once))
        } else {
            Err(fault("trigger.once", "Give once alone, or at with zone."))
        };
    }
    let at = at
        .and_then(|at| NaiveTime::parse_from_str(at, TIME_FORM).ok())
        .ok_or(fault("trigger.at", "Give a time as HH:MM."))?;
    let zone: Tz = zone.and_then(|zone| zone.parse().ok()).ok_or(fault(
        "trigger.zone",
        "Give a time zone, such as Europe/Amsterdam.",
    ))?;
    let days = days
        .iter()
        .map(|name| day(name))
        .collect::<Option<Vec<Weekday>>>()
        .ok_or(fault(
            "trigger.days",
            "Give days as mon, tue, wed, thu, fri, sat or sun.",
        ))?;
    Ok(TimeTrigger::Daily { at, days, zone })
}

fn source_event(source: String, kind: String, contains: Option<String>) -> Result<Trigger, Fault> {
    if !is_word(&source) || !is_word(&kind) {
        return Err(fault(
            "trigger.source",
            "Give a source and a kind in lower snake case.",
        ));
    }
    let contains = contains.filter(|text| !text.is_empty());
    if contains
        .as_ref()
        .is_some_and(|text| text.chars().count() > 200)
    {
        return Err(fault("trigger.contains", "Give at most 200 characters."));
    }
    Ok(Trigger::SourceEvent {
        source,
        kind,
        contains,
    })
}

fn expiry(subject: String, days_before: Vec<u32>) -> Result<Trigger, Fault> {
    let days_valid = !days_before.is_empty()
        && days_before.len() <= 12
        && days_before.iter().all(|days| *days <= 3650);
    if !is_dotted(&subject) || !days_valid {
        return Err(fault(
            "trigger.days_before",
            "Give a subject as extension.name and 1 to 12 numbers of days, each at most 3650.",
        ));
    }
    Ok(Trigger::Expiry {
        subject,
        days_before,
    })
}

fn checked(stored: StoredTrigger) -> Result<Trigger, Fault> {
    match stored {
        StoredTrigger::Time {
            at,
            days,
            zone,
            once,
        } => Ok(Trigger::Time(time(
            at.as_deref(),
            &days,
            zone.as_deref(),
            once,
        )?)),
        StoredTrigger::Place { place, on } => Ok(Trigger::Place {
            place: place.into(),
            on: match on {
                On::Arrive => Crossing::Arrive,
                On::Leave => Crossing::Leave,
            },
        }),
        StoredTrigger::SourceEvent {
            source,
            kind,
            contains,
        } => source_event(source, kind, contains),
        StoredTrigger::SourceState { account, state } if STATES.contains(&state.as_str()) => {
            Ok(Trigger::SourceState {
                account: account.and_then(account_reference),
                state,
            })
        }
        StoredTrigger::SourceState { .. } => Err(fault(
            "trigger.state",
            "Give connected, stalled or needs_reconnect.",
        )),
        StoredTrigger::Threshold { key, is, value } if is_dotted(&key) && value.is_finite() => {
            Ok(Trigger::Threshold {
                key,
                is: is.into(),
                value,
            })
        }
        StoredTrigger::Threshold { .. } => Err(fault(
            "trigger.key",
            "Give a key as extension.name and a number.",
        )),
        StoredTrigger::Expiry {
            subject,
            days_before,
        } => expiry(subject, days_before),
    }
}

/// The trigger that `json` holds.
///
/// # Errors
/// The field to fix.
pub fn trigger(json: &Value) -> Result<Trigger, Fault> {
    let stored: StoredTrigger = serde_json::from_value(json.clone())
        .map_err(|_| fault("trigger", "Give one trigger with its type and its fields."))?;
    checked(stored)
}

/// The `unless` that `json` holds.
///
/// # Errors
/// The field to fix.
pub fn unless(json: &Value) -> Result<Unless, Fault> {
    let bad = fault(
        "unless",
        "Give record_today with source and kind, or measure with key, is and value.",
    );
    let stored: StoredUnless = serde_json::from_value(json.clone()).map_err(|_| bad)?;
    Ok(match stored {
        StoredUnless::RecordToday { source, kind } if is_word(&source) && is_word(&kind) => {
            Unless::RecordToday { source, kind }
        }
        StoredUnless::Measure { key, is, value } if is_dotted(&key) && value.is_finite() => {
            Unless::Measure {
                key,
                is: is.into(),
                value,
            }
        }
        _ => return Err(bad),
    })
}
