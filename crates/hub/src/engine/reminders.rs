//! Reminders: the evaluation of a rule, as a pure function (K-E6 Sections 1 to 4; REM-001,
//! REM-003, REM-004, REM-006). An extension gives occurrences and never changes this code
//! (ENG-001). `notice` evaluates the rules that wait for occurrences in the transaction that
//! stores them, and `claim_send` is the check at send time (REM-002, REM-005). The sweep, the
//! K1 fields and the delivery are not built yet.

mod codec;
mod evaluation;
mod model;
mod place_state;
mod schedule;
mod sending;
mod store;
#[cfg(test)]
mod tests;

pub use codec::{Fault, trigger, trigger_json, unless, unless_json};
pub use evaluation::{day_start, evaluate};
pub use model::{
    Class, Comparison, Crossing, Due, History, Known, Limits, Moment, Occurrence, PlaceChange,
    Rule, Stop, TimeTrigger, Trigger, Unless,
};
pub use place_state::{PlaceState, place_changes};
pub use sending::{RULE_CHANGED, Sendable, claim_send, notice, waiting};
pub use store::StoredRule;

/// Why a reminders operation failed. The message holds no text of a rule or of a record.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// Stores a new rule in the caller's transaction. The K1 part checks the texts of the rule
/// before it calls this.
///
/// # Errors
/// `Database`.
pub async fn add_rule(conn: &mut sqlx::PgConnection, rule: &StoredRule) -> Result<(), Error> {
    Ok(store::insert_rule(conn, rule).await?)
}
