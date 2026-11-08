//! The delivery log (NOT-004) and its day partitions (DB-004, D-25).

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use super::rows::{DELIVERY_COLUMNS, DeliveryRow};
use crate::{
    ids::ChannelId,
    notifications::{
        Error,
        model::{Delivery, DeliveryCursor},
    },
};

/// The database clock, which bounds every "now" a caller passes (CQ-021).
pub(in crate::notifications) async fn database_now(db: &PgPool) -> Result<DateTime<Utc>, Error> {
    Ok(sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(db)
        .await?)
}

/// Creates the delivery partitions of the UTC days from `first` through `days` days after it,
/// in one statement (DB-004).
pub(in crate::notifications) async fn create_days_ahead(
    db: &PgPool,
    first: DateTime<Utc>,
    days: i32,
) -> Result<(), Error> {
    sqlx::query(
        "SELECT notifications.ensure_day_partition(day) \
         FROM generate_series($1::timestamptz, $1::timestamptz + make_interval(days => $2), \
                              interval '1 day') AS day",
    )
    .bind(first)
    .bind(days)
    .execute(db)
    .await?;
    Ok(())
}

/// Drops the delivery partitions of the UTC days before `cutoff`; returns their names. The
/// database refuses a cutoff inside the shortest retention, 7 days (D-25).
pub(in crate::notifications) async fn drop_days_before(
    db: &PgPool,
    cutoff: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<String>, Error> {
    Ok(
        sqlx::query_scalar("SELECT notifications.drop_day_partitions_before($1, $2)")
            .bind(cutoff)
            .bind(now)
            .fetch_all(db)
            .await?,
    )
}

/// Records a delivery, creating its day's partition first when it is missing. Takes a
/// connection so the caller can finish the outbox row in the same transaction.
pub(in crate::notifications) async fn insert_delivery(
    conn: &mut PgConnection,
    delivery: &Delivery,
) -> Result<(), Error> {
    sqlx::query("SELECT notifications.ensure_day_partition($1)")
        .bind(delivery.at)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO notifications.deliveries \
         (id, channel_id, channel_name, kind, at, ok, message) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(delivery.id)
    .bind(delivery.channel)
    .bind(&delivery.channel_name)
    .bind(delivery.kind.as_str())
    .bind(delivery.at)
    .bind(delivery.ok)
    .bind(&delivery.message)
    .execute(conn)
    .await?;
    Ok(())
}

/// The newest delivery of each of `channels`; a channel with none has no entry.
pub(in crate::notifications) async fn last_deliveries(
    db: impl PgExecutor<'_>,
    channels: &[ChannelId],
) -> Result<HashMap<ChannelId, Delivery>, Error> {
    let ids: Vec<Uuid> = channels.iter().map(|id| Uuid::from(*id)).collect();
    let rows: Vec<DeliveryRow> = sqlx::query_as(&format!(
        "SELECT DISTINCT ON (channel_id) {DELIVERY_COLUMNS} FROM notifications.deliveries \
         WHERE channel_id = ANY($1) ORDER BY channel_id, at DESC, id DESC"
    ))
    .bind(ids)
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| {
            let delivery = Delivery::try_from(row)?;
            delivery
                .channel
                .map(|channel| (channel, delivery))
                .ok_or(Error::Unreadable)
        })
        .collect()
}

/// Up to `limit` deliveries, newest first, after `after`; of one channel when `channel` is set.
pub(in crate::notifications) async fn deliveries_page(
    db: impl PgExecutor<'_>,
    channel: Option<ChannelId>,
    after: Option<DeliveryCursor>,
    limit: i64,
) -> Result<Vec<Delivery>, Error> {
    let rows: Vec<DeliveryRow> = sqlx::query_as(&format!(
        "SELECT {DELIVERY_COLUMNS} FROM notifications.deliveries \
         WHERE ($1::uuid IS NULL OR channel_id = $1) \
           AND ($2::timestamptz IS NULL OR (at, id) < ($2, $3)) \
         ORDER BY at DESC, id DESC LIMIT $4"
    ))
    .bind(channel)
    .bind(after.map(|cursor| cursor.at))
    .bind(after.map(|cursor| Uuid::from(cursor.id)))
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Delivery::try_from).collect()
}
