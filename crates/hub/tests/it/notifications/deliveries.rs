//! `NotificationChannel.lastDelivery`: the newest delivery of each channel, batched, with the
//! reason as a fixed word (NOT-004, API-005, CR-14).

use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use anima_hub::{
    clock::{Clock, FixedClock},
    notifications::graphql::{Mutation, Query, last_delivery_loader},
};
use async_graphql::{EmptySubscription, Schema};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::harness::{
    FIELDS, TestSchema, channels, create, data, instant, now, schema_for, webhook,
};
use crate::devices::batching::counting_pool;

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

/// How long the counted loader waits for a batch: long enough that a loaded machine cannot
/// split one, as the devices batching tests do.
const BATCH_WINDOW: Duration = Duration::from_millis(20);

/// The list and one batch of last deliveries cost 2. A read per channel costs 1 + N, so 31
/// for 30 channels; the bound leaves room for a batch a loaded machine splits.
const BATCHED_AT_MOST: usize = 4;

/// The statements `{ notificationChannels { lastDelivery } }` costs, counted at the wire.
async fn statements_for_list(schema: &TestSchema, count: &std::sync::atomic::AtomicUsize) -> usize {
    count.store(0, Ordering::SeqCst);
    let query = format!("{{ notificationChannels {{ {FIELDS} }} }}");
    data(schema, &query, json!({})).await;
    count.load(Ordering::SeqCst)
}

#[sqlx::test(migrations = "./migrations")]
async fn api_005_last_delivery_costs_one_query_for_3_or_30_channels(pool: PgPool) {
    let writer = schema_for(&pool);
    let first = create(&writer, webhook("Channel 00")).await;
    delivered(&pool, &first["id"], now() - TimeDelta::hours(1), None).await;
    for index in 1..3 {
        create(&writer, webhook(&format!("Channel {index:02}"))).await;
    }
    let (counted, count) = counting_pool(&pool).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let schema: TestSchema = Schema::build(Query, Mutation, EmptySubscription)
        .data(counted.clone())
        .data(clock)
        .data(last_delivery_loader(counted).delay(BATCH_WINDOW))
        .finish();
    // The first request opens the pool's connections; the counted ones reuse them.
    statements_for_list(&schema, &count).await;

    let few = statements_for_list(&schema, &count).await;
    for index in 3..30 {
        create(&writer, webhook(&format!("Channel {index:02}"))).await;
    }
    let many = statements_for_list(&schema, &count).await;

    let listed = channels(&schema).await;
    assert_eq!(listed.len(), 30);
    assert_eq!(listed[0]["lastDelivery"]["channelId"], first["id"]);
    assert!(
        few <= BATCHED_AT_MOST && many <= BATCHED_AT_MOST,
        "{few} statements for 3 channels, {many} for 30"
    );
}
