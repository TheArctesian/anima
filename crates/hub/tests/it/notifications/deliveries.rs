//! `NotificationChannel.lastDelivery`: the newest delivery of each channel, batched, with the
//! reason as a fixed word (NOT-004, API-005, CR-14).

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::harness::{channels, create, instant, now, schema_for, webhook};

async fn delivered(pool: &PgPool, channel: &Value, at: DateTime<Utc>, reason: Option<&str>) {
    let channel: Uuid = channel.as_str().unwrap().parse().unwrap();
    sqlx::query("SELECT notifications.ensure_day_partition($1)")
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO notifications.deliveries \
         (id, channel_id, channel_name, kind, at, ok, reason, code) \
         VALUES ($1, $2, 'Ops hook', 'down', $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(channel)
    .bind(at)
    .bind(reason.is_none())
    .bind(reason)
    .bind(reason.map(|_| 503))
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_each_channel_shows_its_newest_delivery(pool: PgPool) {
    let schema = schema_for(&pool);
    let first = create(&schema, webhook("First")).await;
    let second = create(&schema, webhook("Second")).await;
    delivered(&pool, &first["id"], now() - TimeDelta::hours(2), None).await;
    delivered(
        &pool,
        &first["id"],
        now() - TimeDelta::hours(1),
        Some("http_status"),
    )
    .await;

    let listed = channels(&schema).await;
    let last = &listed[0]["lastDelivery"];
    assert_eq!(listed[0]["id"], first["id"]);
    assert_eq!(last["channelId"], first["id"]);
    assert_eq!(last["channelName"], "Ops hook");
    assert_eq!(last["kind"], "DOWN");
    assert_eq!(instant(&last["at"]), now() - TimeDelta::hours(1));
    assert_eq!(last["ok"], false);
    assert_eq!(last["reason"], "HTTP_STATUS");
    assert_eq!(last["code"], 503);
    assert_eq!(listed[1]["id"], second["id"]);
    assert_eq!(listed[1]["lastDelivery"], Value::Null);
}
