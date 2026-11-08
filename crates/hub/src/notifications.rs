//! Notifications (NOT-001 to NOT-007): the channels an operator sets up, the decision of what
//! to send when a check changes state, and the deliveries that send it.
//!
//! Every notification connection leaves the hub through one path, `egress`: the target rule
//! and the deny list judge each resolved address, the connection goes only to a checked
//! address, and TLS is always verified (P3-003, D-17).

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the dispatcher (WP-45 T6) is the first caller")
)]
mod egress;
mod model;
mod store;
#[cfg(test)]
mod tests;

use chrono::{DateTime, Days, DurationRound, NaiveDate, TimeDelta, Utc};
use sqlx::PgPool;

pub use model::{
    Channel, ChannelFields, ChannelKind, CheckSettings, CheckState, Delivery, DeliveryCursor,
    ExpiryMark, InstallationSettings, NewOutbox, NotificationKind, Outbox, OutboxId,
};

/// How many days of delivery partitions exist ahead of today (DB-004).
const DAYS_AHEAD: i32 = 7;

/// How long a finished outbox row stays before clean-up (K2).
const OUTBOX_KEPT: TimeDelta = TimeDelta::days(7);

/// Why a notifications operation failed. Messages hold no SQL and no input value.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error")]
    Database(sqlx::Error),
    /// Another channel already has the name.
    #[error("a channel with this name already exists")]
    NameTaken,
    /// A check was linked to a channel that does not exist.
    #[error("the channel does not exist")]
    UnknownChannel,
    /// A channel was linked to a check that does not exist.
    #[error("the check does not exist")]
    UnknownCheck,
    /// A stored row holds a value this version does not know.
    #[error("a stored notification row cannot be read")]
    Unreadable,
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        match error
            .as_database_error()
            .and_then(|database| database.constraint())
        {
            Some("channels_name_key") => Self::NameTaken,
            Some("check_channels_channel_id_fkey") => Self::UnknownChannel,
            Some("check_channels_check_id_fkey") => Self::UnknownCheck,
            _ => Self::Database(error),
        }
    }
}

impl Error {
    /// For the process log: what failed, as a fixed word, never the source text (LOG-007).
    #[must_use]
    pub const fn fault(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::NameTaken => "name_taken",
            Self::UnknownChannel => "unknown_channel",
            Self::UnknownCheck => "unknown_check",
            Self::Unreadable => "unreadable",
        }
    }
}

/// Creates the delivery partitions of today (UTC) and the next 7 days (DB-004). Wiring only:
/// the daily job calls it.
///
/// # Errors
/// `Database` when a query fails, for instance when `now` is later than the database clock by
/// more than the partition window allows.
pub async fn create_partitions_ahead(db: &PgPool, now: DateTime<Utc>) -> Result<(), Error> {
    let today = now.duration_trunc(TimeDelta::days(1)).unwrap_or(now);
    store::create_days_ahead(db, today, DAYS_AHEAD).await
}

/// Drops the delivery partitions older than `keep` days before `now` (the `retention` period of
/// deliveries, D-25), and returns their names. `now` is held to the database clock, and the
/// database refuses to drop a partition less than 7 days old. Wiring only: the daily job calls
/// it.
///
/// # Errors
/// `Database` when a query fails or the database refuses the cutoff.
pub async fn drop_expired_deliveries(
    db: &PgPool,
    now: DateTime<Utc>,
    keep: Days,
) -> Result<Vec<String>, Error> {
    let now = now.min(store::database_now(db).await?);
    let cutoff = now
        .date_naive()
        .checked_sub_days(keep)
        .unwrap_or(NaiveDate::MIN);
    store::drop_days_before(db, cutoff, now).await
}

/// Deletes the outbox rows sent or given up more than 7 days before `now`; returns how many
/// (K2). Wiring only: the daily job calls it.
///
/// # Errors
/// `Database` when the query fails.
pub async fn delete_finished_outbox(db: &PgPool, now: DateTime<Utc>) -> Result<u64, Error> {
    store::delete_finished_before(db, now - OUTBOX_KEPT).await
}
