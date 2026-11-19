//! The read of a follower of the event log: the events after an instant, oldest first.

use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use super::EventRow;
use crate::fleet::events::EventSummary;

/// The events after `after` and not after `until`, oldest first, at most `limit`.
pub(in crate::fleet::events) async fn events_after(
    db: impl PgExecutor<'_>,
    (after, until): (DateTime<Utc>, DateTime<Utc>),
    limit: i64,
) -> Result<Vec<EventSummary>, sqlx::Error> {
    let rows: Vec<EventRow> = sqlx::query_as(
        "select id, at, severity, source, device_id, device_name, message, actor_kind \
         from events.event_log where at > $1 and at <= $2 order by at, id limit $3",
    )
    .bind(after)
    .bind(until)
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(EventSummary::try_from).collect()
}
