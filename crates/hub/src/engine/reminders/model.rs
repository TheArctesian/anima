//! What a rule, an occurrence and a firing are (K-E6 Sections 1 to 3). Pure.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;

use crate::{
    engine::records::Reference,
    ids::{FiringId, PlaceId, ReminderRuleId, SampleId},
};

/// The class of the thing that an occurrence is about: A, B or C (K-E5; K-E6 names it
/// `DataClass`). The engine uses it only
/// for the text on a lock screen (REM-009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    A,
    B,
    C,
}

/// What an extension or the engine gives to the reminders (K-E6 Section 1).
#[derive(Clone, Debug, PartialEq)]
pub enum Occurrence {
    /// A record that a source or a domain stored.
    Record {
        source: String,
        kind: String,
        record: Reference,
        title: String,
        class: Class,
        at: DateTime<Utc>,
    },
    /// A number at a time.
    Measure {
        key: String,
        value: f64,
        class: Class,
        at: DateTime<Utc>,
    },
    /// A thing that ends at a time.
    Expiry {
        subject: String,
        record: Reference,
        title: String,
        class: Class,
        expires_at: DateTime<Utc>,
    },
}

/// Below or above a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    Below,
    Above,
}

impl Comparison {
    /// Whether `measured` is on this side of `value`. A value that is equal is on no side.
    #[must_use]
    pub fn holds(self, measured: f64, value: f64) -> bool {
        match self {
            Self::Below => measured < value,
            Self::Above => measured > value,
        }
    }
}

/// The change of a place state that a place rule waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossing {
    Arrive,
    Leave,
}

/// When a time rule is due.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeTrigger {
    /// At a local time of `zone`, on each day of `days` (each day when empty).
    Daily {
        at: NaiveTime,
        days: Vec<Weekday>,
        zone: Tz,
    },
    /// One time, at an instant.
    Once(DateTime<Utc>),
}

/// The one trigger of a rule (K-E6 Section 2).
#[derive(Clone, Debug, PartialEq)]
pub enum Trigger {
    Time(TimeTrigger),
    Place {
        place: PlaceId,
        on: Crossing,
    },
    /// A `Record` with this source and kind whose title contains `contains`, in any case.
    SourceEvent {
        source: String,
        kind: String,
        contains: Option<String>,
    },
    /// The health of a source account changes to `state`; `account` is `None` for any account.
    /// The source framework gives the change as a `Record` with the source `sources`, the
    /// state as its kind and the account as its record.
    SourceState {
        account: Option<Reference>,
        state: String,
    },
    Threshold {
        key: String,
        is: Comparison,
        value: f64,
    },
    /// An `Expiry` with this subject is not more than N days away, one time for each N.
    Expiry {
        subject: String,
        days_before: Vec<u32>,
    },
}

/// The one condition that stops a reminder (REM-003).
#[derive(Clone, Debug, PartialEq)]
pub enum Unless {
    /// A `Record` with this source and kind is known for the day of the rule.
    RecordToday { source: String, kind: String },
    /// The newest measure of the key is on this side of the value.
    Measure {
        key: String,
        is: Comparison,
        value: f64,
    },
}

/// How often a rule may fire (REM-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub cooldown_minutes: u32,
    pub daily_limit: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            cooldown_minutes: 60,
            daily_limit: 3,
        }
    }
}

/// One rule, as the evaluation reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: ReminderRuleId,
    pub name: String,
    /// The text of the reminder; `{title}` is the title of the occurrence.
    pub message: String,
    pub trigger: Trigger,
    pub unless: Option<Unless>,
    pub limits: Limits,
    pub revision: i32,
    /// The last change of the rule: a time before it is not due for this revision.
    pub updated_at: DateTime<Utc>,
}

/// What the rule sent before: the last firing that was not suppressed, how many of them the
/// day of the rule has, and the subjects that fired.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct History {
    pub last_fired: Option<DateTime<Utc>>,
    pub fired_today: u32,
    pub subjects: HashSet<String>,
}

/// What the hub knew before this moment: the records of the day of the rule, as
/// `(source, kind)`, and the newest measure of each key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Known {
    pub records_today: HashSet<(String, String)>,
    pub measures: HashMap<String, f64>,
}

/// A place whose state changed in this sweep, with the sample that changed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaceChange {
    pub place: PlaceId,
    pub inside: bool,
    pub sample: SampleId,
    pub observed_at: DateTime<Utc>,
}

/// One evaluation: the time, the new occurrences, the place changes and what was known.
#[derive(Clone, Copy, Debug)]
pub struct Moment<'a> {
    pub now: DateTime<Utc>,
    pub new: &'a [Occurrence],
    pub places: &'a [PlaceChange],
    pub known: &'a Known,
}

/// Why a firing that was due is not sent. It is stored, so that the operator sees it; it is
/// not sent later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Cooldown,
    DailyLimit,
}

impl Stop {
    /// The text stored in `reminders.firing.reason`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cooldown => "cooldown",
            Self::DailyLimit => "daily_limit",
        }
    }
}

/// One firing of a rule. `stopped` is `None` for a firing to send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due {
    pub rule: ReminderRuleId,
    pub revision: i32,
    /// What the firing is about; one rule fires at most one time for one subject.
    pub subject: String,
    pub text: String,
    pub class: Class,
    pub stopped: Option<Stop>,
}

/// The state of a firing (K-E6 Section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiringState {
    Pending,
    Sent,
    Suppressed,
    Snoozed,
    Acknowledged,
}

impl FiringState {
    /// The text stored in `reminders.firing.state`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Suppressed => "suppressed",
            Self::Snoozed => "snoozed",
            Self::Acknowledged => "acknowledged",
        }
    }

    /// The state whose stored text is `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [
            Self::Pending,
            Self::Sent,
            Self::Suppressed,
            Self::Snoozed,
            Self::Acknowledged,
        ]
        .into_iter()
        .find(|state| state.as_str() == text)
    }
}

/// One stored firing, as the operator reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Firing {
    pub id: FiringId,
    pub rule: ReminderRuleId,
    pub revision: i32,
    pub subject: String,
    pub text: String,
    pub class: Class,
    pub state: FiringState,
    /// Why a suppressed firing was not sent.
    pub reason: Option<String>,
    pub due_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub decided_at: Option<DateTime<Utc>>,
}
