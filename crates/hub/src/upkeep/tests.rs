//! The daily retention run reads the operator's periods once and gives each domain its own
//! (HUB-005, D-25), also a period the operator set through `setRetention`.

use std::sync::Arc;

use anima_hub::{
    access::Identity,
    clock::{Clock, FixedClock},
    events::{Bus, ClientAddr, Operator},
    ids::OperatorId,
    retention::graphql::{Mutation, Query},
};
use async_graphql::{EmptySubscription, Request, Schema};
use chrono::{DateTime, Days, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::keep;

/// The name of the day partition of `table` for the day `days_ago` days before `now`.
fn partition(table: &str, now: DateTime<Utc>, days_ago: u64) -> String {
    let day = now - Days::new(days_ago);
    format!("{table}_{}", day.format("%Y%m%d"))
}

async fn make_partitions(db: &PgPool, now: DateTime<Utc>, days_ago: u64) {
    let day = now - Days::new(days_ago);
    for schema in ["checks", "metrics", "notifications"] {
        sqlx::query(&format!("select {schema}.ensure_day_partition($1)"))
            .bind(day)
            .execute(db)
            .await
            .unwrap();
    }
    sqlx::query("select events.ensure_day_partition('event_log', $1)")
        .bind(day)
        .execute(db)
        .await
        .unwrap();
}

async fn exists(db: &PgPool, name: &str) -> bool {
    sqlx::query_scalar("select to_regclass($1) is not null")
        .bind(name)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn set_period(db: &PgPool, subject: &str, days: i32) {
    sqlx::query("update retention.periods set days = $2 where subject = $1")
        .bind(subject)
        .bind(days)
        .execute(db)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_an_unreadable_period_skips_the_run_and_drops_nothing(db: PgPool) {
    let now = Utc::now();
    make_partitions(&db, now, 10).await;
    set_period(&db, "heartbeats", 5).await;
    set_period(&db, "metrics", 5).await;
    sqlx::query("delete from retention.periods where subject = 'events'")
        .execute(&db)
        .await
        .unwrap();

    let outcome = keep(&db, now).await;

    assert_eq!(outcome, Err("unreadable"));
    for table in ["checks.heartbeats", "metrics.samples"] {
        assert!(exists(&db, &partition(table, now, 10)).await, "{table}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_each_domain_drops_by_its_own_period(db: PgPool) {
    let now = Utc::now();
    make_partitions(&db, now, 10).await;
    set_period(&db, "heartbeats", 5).await;
    set_period(&db, "metrics", 15).await;
    set_period(&db, "events", 30).await;
    set_period(&db, "deliveries", 7).await;

    keep(&db, now).await.unwrap();

    assert!(!exists(&db, &partition("notifications.deliveries", now, 10)).await);

    assert!(!exists(&db, &partition("checks.heartbeats", now, 10)).await);
    assert!(exists(&db, &partition("metrics.samples", now, 10)).await);
    assert!(exists(&db, &partition("events.event_log", now, 10)).await);

    make_partitions(&db, now, 10).await;
    set_period(&db, "heartbeats", 15).await;
    set_period(&db, "metrics", 5).await;
    set_period(&db, "deliveries", 15).await;

    keep(&db, now).await.unwrap();

    assert!(exists(&db, &partition("notifications.deliveries", now, 10)).await);

    assert!(exists(&db, &partition("checks.heartbeats", now, 10)).await);
    assert!(!exists(&db, &partition("metrics.samples", now, 10)).await);
}

/// Sends `setRetention` for `subject` as a signed-in operator at `now`.
async fn set_retention(db: &PgPool, now: DateTime<Utc>, subject: &str, days: i32) {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(db.clone())
        .data(Bus::new(16))
        .data(clock)
        .finish();
    let operator = Operator {
        id: OperatorId::from(Uuid::now_v7()),
        name: "operator".to_owned(),
        actor: crate::events::Actor::for_tests(),
    };
    let request = Identity::for_tests(operator, now + TimeDelta::days(1))
        .attach(Request::new(format!(
            "mutation {{ setRetention(subject: {subject}, days: {days}) {{ days }} }}"
        )))
        .data(ClientAddr("10.10.0.77".parse().unwrap()));
    let response = schema.execute(request).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
}

async fn event_at(db: &PgPool, at: DateTime<Utc>, message: &str) {
    sqlx::query(
        "insert into events.event_log (id, at, severity, source, message) \
         values ($1, $2, 'info', 'hub', $3)",
    )
    .bind(Uuid::now_v7())
    .bind(at)
    .bind(message)
    .execute(db)
    .await
    .unwrap();
}

async fn event_messages(db: &PgPool, like: &str) -> Vec<String> {
    sqlx::query_scalar("select message from events.event_log where message like $1 order by at")
        .bind(like)
        .fetch_all(db)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn hub_005_a_period_set_through_set_retention_applies_at_the_next_daily_run(db: PgPool) {
    let now = Utc::now();
    for days_ago in [40, 20] {
        sqlx::query("select events.ensure_day_partition('event_log', $1)")
            .bind(now - Days::new(days_ago))
            .execute(&db)
            .await
            .unwrap();
    }
    event_at(&db, now - Days::new(40), "row of 40 days").await;
    event_at(&db, now - Days::new(20), "row of 20 days").await;
    keep(&db, now).await.unwrap();
    assert_eq!(
        event_messages(&db, "row of%").await,
        ["row of 40 days", "row of 20 days"],
        "the default of 90 days keeps both"
    );

    set_retention(&db, now, "EVENTS", 30).await;
    assert_eq!(
        event_messages(&db, "row of%").await.len(),
        2,
        "setting a period drops nothing at once"
    );
    keep(&db, now).await.unwrap();

    assert_eq!(event_messages(&db, "row of%").await, ["row of 20 days"]);
}
