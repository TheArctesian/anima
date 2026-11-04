//! The event log and the audit log: what happened and who changed what (LOG-004, LOG-005),
//! and the live broadcast of new events and status changes (LOG-010).

mod bus;
mod model;
pub mod process_log;
mod redaction;
mod retention;
mod store;
#[cfg(test)]
mod tests;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

pub use bus::{Bus, Signal};
pub use model::{
    AuditAction, AuditTarget, ClientAddr, EventSummary, HeartbeatSummary, NewAudit, NewEvent,
    Operator, Severity, Source,
};

use crate::ids::EventId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("the event store is unavailable")]
    Database(#[from] sqlx::Error),
}

/// Why the hub must not serve with its database role (K2 "Roles", R-016, LOG-006). Only
/// `check_runtime_role` returns it, once at start before the hub serves, so it never reaches
/// GraphQL; the messages are plain sentences for the operator at the console (API-010).
#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("The hub could not check its database role.")]
    Database(#[from] sqlx::Error),
    #[error("The events schema is missing. Run the migrate subcommand before serving.")]
    NotMigrated,
    #[error("The hub's database role is a superuser. Serve as a member of anima_runtime instead.")]
    Superuser,
    #[error(
        "The hub's database role owns the event log or the audit log. Run migrations as the \
         owner role and serve as a member of anima_runtime."
    )]
    OwnsLogs,
    #[error(
        "The hub's database role owns the events schema. Run migrations as the owner role and \
         serve as a member of anima_runtime."
    )]
    OwnsEventsSchema,
    #[error(
        "The hub's database role can change or remove log entries. Revoke its UPDATE, DELETE, \
         TRUNCATE and TRIGGER rights on the logs and CREATE on the events schema, and serve as \
         a member of anima_runtime."
    )]
    CanChangeLogs,
}

/// Checks that the hub serves as a runtime role that cannot change the logs, nor become a
/// role that can with `SET ROLE` (K2 "Roles", R-016, LOG-006). It refuses a superuser, an
/// owner of a log table, a partition or schema `events`, and a holder of UPDATE, DELETE,
/// TRUNCATE or TRIGGER on a log table or partition (directly, through `pg_write_all_data` or
/// PUBLIC) or of CREATE on `events`. Call at start, before serving.
///
/// # Errors
///
/// A `RoleError` refusal with a plain message (API-010); `RoleError::Database` if the check
/// cannot run.
pub async fn check_runtime_role(db: &PgPool) -> Result<(), RoleError> {
    let role = store::runtime_role(db)
        .await?
        .ok_or(RoleError::NotMigrated)?;
    if role.superuser {
        Err(RoleError::Superuser)
    } else if role.owns_logs {
        Err(RoleError::OwnsLogs)
    } else if role.owns_events_schema {
        Err(RoleError::OwnsEventsSchema)
    } else if role.can_change_logs {
        Err(RoleError::CanChangeLogs)
    } else {
        Ok(())
    }
}

/// Drops the day partitions older than each log's retention at `now` (90 days for events,
/// 1 year for the audit log; DB-005) and returns the names of the dropped partitions. The
/// database refuses to drop anything inside retention, by `now` and by its own clock
/// (LOG-006). Schedule it daily.
///
/// # Errors
///
/// `Error::Database` if a partition cannot be dropped or the database refuses the cutoff.
pub async fn drop_expired_partitions(
    db: &PgPool,
    now: DateTime<Utc>,
) -> Result<Vec<String>, Error> {
    let mut dropped = Vec::new();
    for log in retention::LOGS {
        let first_kept_day = log.first_kept_day(now);
        dropped.extend(store::drop_day_partitions_before(db, log, first_kept_day, now).await?);
    }
    Ok(dropped)
}

/// Creates the day partitions of both logs for `today` and the `days` days after it, each in
/// its own short transaction (D-4).
///
/// Call it at start and daily, so the partition already exists when `record` or `audit` runs.
/// Both still create a missing partition as a fallback, but that holds an `ACCESS EXCLUSIVE`
/// lock on the parent table until the caller's transaction ends, which blocks readers and can
/// deadlock two transactions that write both logs in opposite order. With the partition
/// already there, an open writer does not block readers and the two orders do not deadlock
/// (both tested).
///
/// # Errors
///
/// `Error::Database` if a partition cannot be created.
pub async fn create_partitions_ahead(
    db: &PgPool,
    today: DateTime<Utc>,
    days: u32,
) -> Result<(), Error> {
    for offset in 0..=days {
        store::create_day_partitions(db, today + TimeDelta::days(i64::from(offset))).await?;
    }
    Ok(())
}

/// Stores the event, then publishes it on `bus` as `Signal::Event`.
///
/// `message` is stored and broadcast as is: it must never hold a password, key, token or
/// session content (LOG-007).
///
/// If the day partition is missing, it is created inside the caller's transaction (see
/// `create_partitions_ahead` for the lock this takes).
///
/// The publish happens inside the caller's transaction: if the caller rolls back, subscribers
/// have seen an event that is not stored (D-5). The subscription is a live view; the stored
/// event list is the truth.
///
/// # Errors
///
/// `Error::Database` if the partition or the row cannot be written.
pub async fn record(
    conn: &mut PgConnection,
    bus: &Bus,
    at: DateTime<Utc>,
    event: NewEvent,
) -> Result<EventId, Error> {
    let summary = store::insert_event(conn, at, event).await?;
    let id = summary.id;
    bus.publish(Signal::Event(summary));
    Ok(id)
}

/// Stores an audit entry. Secret values in `change` are replaced with `"[redacted]"` first
/// (LOG-007); callers should not pass secrets, this is the backstop.
///
/// If the day partition is missing, it is created inside the caller's transaction (see
/// `create_partitions_ahead` for the lock this takes).
///
/// # Errors
///
/// `Error::Database` if the partition or the row cannot be written.
pub async fn audit(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    entry: NewAudit,
) -> Result<(), Error> {
    let entry = NewAudit {
        change: redaction::redact(entry.change),
        ..entry
    };
    store::insert_audit(conn, at, &entry).await?;
    Ok(())
}
