//! Reminders: the evaluation of a rule, as a pure function (K-E6 Sections 1 to 4; REM-001,
//! REM-003, REM-004, REM-006). An extension gives occurrences and never changes this code
//! (ENG-001). `notice` evaluates the rules that wait for occurrences in the transaction that
//! stores them, `sweep` evaluates the time rules and the place rules, and `send_waiting` sends each
//! firing through the outbox of `notifications` after the check at send time (REM-002, REM-005,
//! REM-006, REM-010). The other things that the sweep reads and the K1 fields are not built yet.

mod codec;
mod delivery;
mod evaluation;
pub mod graphql;
mod model;
mod occurrences;
mod place_state;
mod rules;
mod schedule;
mod sending;
mod store;
mod sweep;
#[cfg(test)]
mod tests;
mod trail;

pub use codec::{Fault, trigger, trigger_json, unless, unless_json};
pub use delivery::{
    NO_CHANNEL, Sending, TOPIC_NOT_RANDOM, lock_screen, send_waiting, spawn_minute,
};
pub use evaluation::{day_start, evaluate};
pub use model::{
    Class, Comparison, Crossing, Due, Firing, FiringState, History, Known, Limits, Moment,
    Occurrence, PlaceChange, Rule, Stop, TimeTrigger, Trigger, Unless,
};
pub use occurrences::{Extensions, Triggers, Unavailable};
pub use place_state::{PlaceState, place_changes};
pub use rules::{
    Draft, FIRINGS_MAX, MINUTES_MAX, acknowledge, activity, create_rule, firings, remove_rule,
    rule, rules, set_enabled, snooze, test_rule, update_rule,
};
pub use sending::{RULE_CHANGED, Sendable, claim_send, notice, waiting};
pub use store::StoredRule;
pub use sweep::{Reading, Swept, sweep};

/// Why a reminders operation failed. The message holds no text of a rule or of a record.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("the places cannot be read")]
    Places(#[from] crate::engine::places::Error),
    #[error("the location samples cannot be read ({0})")]
    Locations(#[from] crate::engine::locations::Error),
    #[error("the outbox of the notifications cannot be written")]
    Notifications(#[from] crate::fleet::notifications::Error),
    #[error("the event cannot be written")]
    Events(#[from] crate::fleet::events::Error),
    #[error("the operational keys cannot be read")]
    Keys(#[from] crate::fleet::credentials::operational::Error),
    #[error(transparent)]
    Extension(#[from] Unavailable),
    #[error("the source accounts cannot be read")]
    Sources(#[from] crate::engine::sources::Error),
    #[error("the extension switches cannot be read")]
    Switches(#[from] crate::engine::extensions::Error),
    /// Input to fix: each field and what to give.
    #[error("the reminder input is not valid")]
    Invalid(Vec<Fault>),
    #[error("no such {0}")]
    NotFound(&'static str),
    /// A snooze or an acknowledge of a firing that was not sent, or that is decided.
    #[error("the firing was not sent or is decided")]
    NotSent,
}
