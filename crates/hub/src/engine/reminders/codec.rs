//! The stored form of a trigger and of `unless` (K-E6 Section 2): JSON in
//! `reminders.rule.trigger` and `reminders.rule.unless`. Pure. A value that the hub cannot read
//! gives the field to fix, never a part of the value.

use chrono::{DateTime, Utc, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod reading;

pub use reading::{trigger, unless};

use super::model::{Comparison, Crossing, TimeTrigger, Trigger, Unless};

/// One field of a rule to fix, with what to give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fault {
    pub field: &'static str,
    pub message: &'static str,
}

const TIME_FORM: &str = "%H:%M";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Side {
    Below,
    Above,
}

impl From<Side> for Comparison {
    fn from(side: Side) -> Self {
        match side {
            Side::Below => Self::Below,
            Side::Above => Self::Above,
        }
    }
}

impl From<Comparison> for Side {
    fn from(comparison: Comparison) -> Self {
        match comparison {
            Comparison::Below => Self::Below,
            Comparison::Above => Self::Above,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum On {
    Arrive,
    Leave,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum StoredTrigger {
    Time {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        days: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        zone: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        once: Option<DateTime<Utc>>,
    },
    Place {
        place: uuid::Uuid,
        on: On,
    },
    SourceEvent {
        source: String,
        kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        contains: Option<String>,
    },
    SourceState {
        #[serde(default)]
        account: Option<uuid::Uuid>,
        state: String,
    },
    Threshold {
        key: String,
        is: Side,
        value: f64,
    },
    Expiry {
        subject: String,
        days_before: Vec<u32>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum StoredUnless {
    RecordToday { source: String, kind: String },
    Measure { key: String, is: Side, value: f64 },
}

fn day_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

fn day(name: &str) -> Option<Weekday> {
    [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ]
    .into_iter()
    .find(|day| day_name(*day) == name)
}

/// The stored form of `trigger`. A trigger that [`trigger`] gave is written back equal.
#[must_use]
pub fn trigger_json(trigger: &Trigger) -> Value {
    let stored = match trigger {
        Trigger::Time(TimeTrigger::Once(once)) => StoredTrigger::Time {
            at: None,
            days: Vec::new(),
            zone: None,
            once: Some(*once),
        },
        Trigger::Time(TimeTrigger::Daily { at, days, zone }) => StoredTrigger::Time {
            at: Some(at.format(TIME_FORM).to_string()),
            days: days.iter().map(|day| day_name(*day).to_owned()).collect(),
            zone: Some(zone.name().to_owned()),
            once: None,
        },
        Trigger::Place { place, on } => StoredTrigger::Place {
            place: (*place).into(),
            on: match on {
                Crossing::Arrive => On::Arrive,
                Crossing::Leave => On::Leave,
            },
        },
        Trigger::SourceEvent {
            source,
            kind,
            contains,
        } => StoredTrigger::SourceEvent {
            source: source.clone(),
            kind: kind.clone(),
            contains: contains.clone(),
        },
        Trigger::SourceState { account, state } => StoredTrigger::SourceState {
            account: account
                .as_ref()
                .and_then(|account| account.as_str().split_once(':'))
                .and_then(|(_, id)| id.parse().ok()),
            state: state.clone(),
        },
        Trigger::Threshold { key, is, value } => StoredTrigger::Threshold {
            key: key.clone(),
            is: (*is).into(),
            value: *value,
        },
        Trigger::Expiry {
            subject,
            days_before,
        } => StoredTrigger::Expiry {
            subject: subject.clone(),
            days_before: days_before.clone(),
        },
    };
    serde_json::to_value(stored).unwrap_or(Value::Null)
}

/// The stored form of `unless`.
#[must_use]
pub fn unless_json(unless: &Unless) -> Value {
    let stored = match unless {
        Unless::RecordToday { source, kind } => StoredUnless::RecordToday {
            source: source.clone(),
            kind: kind.clone(),
        },
        Unless::Measure { key, is, value } => StoredUnless::Measure {
            key: key.clone(),
            is: (*is).into(),
            value: *value,
        },
    };
    serde_json::to_value(stored).unwrap_or(Value::Null)
}
