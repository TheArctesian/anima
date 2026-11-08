//! The daily retention run reads the operator's periods once and gives each domain its own
//! (HUB-005, D-25).

use chrono::{DateTime, Days, Utc};
use sqlx::PgPool;

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
