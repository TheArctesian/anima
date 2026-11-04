//! The only SQL for `events.event_log` and `events.audit_log`.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::model::{EventSummary, NewAudit, NewEvent};
use crate::ids::EventId;

const ENSURE_DAY_PARTITION: &str = "select events.ensure_day_partition($1, $2)";

/// Creates the UTC-day partition of `parent` that holds `at`, if it does not exist yet (D-4).
/// Inside the caller's transaction.
async fn ensure_day_partition(
    conn: &mut PgConnection,
    parent: &str,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(ENSURE_DAY_PARTITION)
        .bind(parent)
        .bind(at)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Creates the day partitions of both logs that hold `at`. Each call is its own statement
/// outside any transaction, so the lock it takes on a parent ends at once.
pub(super) async fn create_day_partitions(
    db: &PgPool,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    for parent in ["event_log", "audit_log"] {
        sqlx::query(ENSURE_DAY_PARTITION)
            .bind(parent)
            .bind(at)
            .execute(db)
            .await?;
    }
    Ok(())
}

/// Inserts the event and returns it as stored (`at` at the database's microsecond precision).
pub(super) async fn insert_event(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    event: NewEvent,
) -> Result<EventSummary, sqlx::Error> {
    ensure_day_partition(conn, "event_log", at).await?;
    let id = EventId::from(Uuid::now_v7());
    let (device_id, device_name) = event
        .device
        .as_ref()
        .map(|(id, name)| (*id, name.as_str()))
        .unzip();
    let at = sqlx::query_scalar(
        "insert into events.event_log \
         (id, at, severity, source, device_id, device_name, message) \
         values ($1, $2, $3, $4, $5, $6, $7) returning at",
    )
    .bind(id)
    .bind(at)
    .bind(event.severity.as_str())
    .bind(event.source.as_str())
    .bind(device_id)
    .bind(device_name)
    .bind(&event.message)
    .fetch_one(&mut *conn)
    .await?;
    Ok(EventSummary {
        id,
        at,
        severity: event.severity,
        source: event.source,
        device: event.device,
        message: event.message,
    })
}

/// Inserts the audit entry as given; the caller has already redacted `change`.
pub(super) async fn insert_audit(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    entry: &NewAudit,
) -> Result<(), sqlx::Error> {
    ensure_day_partition(conn, "audit_log", at).await?;
    sqlx::query(
        "insert into events.audit_log \
         (id, at, operator_id, operator_name, action, target_type, target_id, client_addr, change) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(Uuid::now_v7())
    .bind(at)
    .bind(entry.operator.id)
    .bind(&entry.operator.name)
    .bind(entry.action.as_str())
    .bind(entry.target.kind())
    .bind(entry.target.id())
    .bind(entry.client.0)
    .bind(&entry.change)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
