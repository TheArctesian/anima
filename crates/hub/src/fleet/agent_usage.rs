//! The use of the AI agent programs on each device: the counts of prompts, tool calls and
//! tokens of each session of Claude Code and Codex by UTC day and by model, as the daemon of
//! the device sends them in its push batch, and what each daemon can read (R-135; AIS-018 to
//! AIS-024; K8 Section 4).
//!
//! [`record`] stores an accepted `agentUsage` member in the transaction of its batch: an
//! upsert, so the same entry sent again changes nothing. [`usage`] sums the stored counts by
//! the dimensions the operator asks for. The pure part: [`prices`] estimates the cost of
//! tokens from a fixed table of public list prices (AIS-021), and [`grouping`] folds the sums
//! by model into the groups asked for. The texts (model and repository names) come from a
//! device: they were checked at ingest and only ever reach SQL as bound values.

pub mod graphql;
mod grouping;
mod model;
pub mod prices;
mod store;
#[cfg(test)]
mod tests;

use chrono::NaiveDate;
use daemon_core::agent_usage::AgentUsage;
use sqlx::{PgConnection, PgPool};

pub use model::{Ask, Dimension, DimensionSet, Report};

use crate::ids::DeviceId;

/// The most groups one query may sum before the hub asks for fewer dimensions or days.
const MAX_GRAINS: usize = 20_000;

/// Why an agent usage operation failed. Messages are fixed text: they hold no SQL and no
/// value from a device.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// A stored source or state this version does not know.
    #[error("a stored agent usage row cannot be read")]
    Unreadable,
    /// The groups asked for are more than the hub sums in one query.
    #[error("too many groups")]
    TooManyGroups,
}

/// Stores `usage`, accepted from a batch of `device`, in the caller's transaction (K8 Section
/// 4): the state when the hub knows it, and each session day of a known source, which replaces
/// what is stored for that session and day. Start, end and repository of a session come from
/// its entry stored last.
///
/// # Errors
/// `Database` when a statement fails; nothing of the batch is then kept.
pub async fn record(
    conn: &mut PgConnection,
    device: DeviceId,
    usage: &AgentUsage,
) -> Result<(), Error> {
    if let Some(state) = usage.state {
        store::write_state(conn, device, state).await?;
    }
    store::write_session_days(conn, device, &usage.session_days).await
}

/// The sums `ask` asks for over the days from `first_day` to `last_day`, both included: one
/// group for each value of the dimensions in `ask.by`, the total, and the usage state of each
/// device the filter admits. Groups are in no set order.
///
/// # Errors
/// `TooManyGroups` past [`MAX_GRAINS`]; `Database` when a query fails; `Unreadable` for a
/// stored source or state this version does not know.
pub async fn usage(db: &PgPool, ask: &Ask) -> Result<Report, Error> {
    let grains = store::grains(db, ask, ask.by, MAX_GRAINS).await?;
    let groups = grouping::fold(grains, ask.by);
    let total_grains = store::grains(db, ask, DimensionSet::default(), MAX_GRAINS).await?;
    let total = grouping::fold(total_grains, DimensionSet::default())
        .pop()
        .unwrap_or_default();
    let states = store::states(db, ask.device).await?;
    Ok(Report {
        groups,
        total,
        states,
    })
}

/// The last day of the default range: `today`, and its first day: 29 days before, so the range
/// holds 30 days (AIS-020).
#[must_use]
pub fn default_days(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let first = today
        .checked_sub_days(chrono::Days::new(model::DEFAULT_DAYS - 1))
        .unwrap_or(today);
    (first, today)
}
