//! The daily partition upkeep: a week of partitions ahead, expired ones dropped, and a lock
//! that is not granted in time fails the run instead of queueing every reader behind it (D-4,
//! DB-004, DB-005).

use std::time::Duration;

use chrono::{DateTime, Days, Utc};
use sqlx::{PgConnection, PgPool};

use super::partitions::table_exists;
use crate::fleet::events::{
    Error, create_partitions_ahead, drop_expired_partitions, keep_partitions,
};

/// The test database's clock: the drop function bounds its cutoff by it.
async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn exists(conn: &mut PgConnection, table: &str, day: DateTime<Utc>) -> bool {
    let name = format!("events.{table}_{}", day.format("%Y%m%d"));
    table_exists(conn, &name).await
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_db_005_keep_partitions_creates_a_week_ahead_and_drops_expired_days(pool: PgPool) {
    let now = database_now(&pool).await;
    let ago = |days| now - Days::new(days);
    for old in [ago(100), ago(80), ago(400), ago(300)] {
        create_partitions_ahead(&pool, old, 0).await.unwrap();
    }

    keep_partitions(&pool, now, Days::new(90)).await.unwrap();

    let mut conn = pool.acquire().await.unwrap();
    for table in ["event_log", "audit_log"] {
        for ahead in 0..=7 {
            let day = now + Days::new(ahead);
            assert!(exists(&mut conn, table, day).await, "{table} +{ahead}");
        }
        assert!(
            !exists(&mut conn, table, now + Days::new(8)).await,
            "{table} +8"
        );
    }
    for (table, day, kept) in [
        ("event_log", ago(100), false),
        ("event_log", ago(80), true),
        ("audit_log", ago(400), false),
        ("audit_log", ago(300), true),
        ("event_log", ago(300), false),
        ("audit_log", ago(100), true),
    ] {
        assert_eq!(exists(&mut conn, table, day).await, kept, "{table} {day}");
    }
}

/// Opens a transaction that reads `events.event_log`, holding its lock until dropped.
async fn reader_holding_the_event_log(pool: &PgPool) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut reader = pool.begin().await.unwrap();
    sqlx::query("select count(*) from events.event_log")
        .execute(&mut *reader)
        .await
        .unwrap();
    reader
}

/// Runs `change` with a deadline well past the lock timeout; its fault word.
async fn fault_within_deadline<F>(change: F) -> &'static str
where
    F: Future<Output = Result<(), Error>>,
{
    let outcome = tokio::time::timeout(Duration::from_secs(20), change)
        .await
        .expect("the change gives up on the lock before the deadline");
    outcome.expect_err("the lock is held").fault()
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_creating_ahead_gives_up_on_a_lock_held_by_a_long_reader(pool: PgPool) {
    let now = database_now(&pool).await;
    let reader = reader_holding_the_event_log(&pool).await;

    let blocked = create_partitions_ahead(&pool, now + Days::new(30), 0);
    assert_eq!(fault_within_deadline(blocked).await, "lock_timeout");

    // The reader went on unharmed; once it ends, the retry succeeds.
    reader.commit().await.unwrap();
    create_partitions_ahead(&pool, now + Days::new(30), 0)
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    assert!(exists(&mut conn, "event_log", now + Days::new(30)).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_005_dropping_expired_days_gives_up_on_a_lock_held_by_a_long_reader(pool: PgPool) {
    let now = database_now(&pool).await;
    create_partitions_ahead(&pool, now - Days::new(100), 0)
        .await
        .unwrap();
    let reader = reader_holding_the_event_log(&pool).await;

    let blocked = async {
        drop_expired_partitions(&pool, now, Days::new(90))
            .await
            .map(drop)
    };
    assert_eq!(fault_within_deadline(blocked).await, "lock_timeout");

    reader.commit().await.unwrap();
    let dropped = drop_expired_partitions(&pool, now, Days::new(90))
        .await
        .unwrap();
    let expected = format!("event_log_{}", (now - Days::new(100)).format("%Y%m%d"));
    assert!(dropped.contains(&expected), "{dropped:?}");
}

#[test]
fn api_010_a_database_fault_is_named_by_kind() {
    assert_eq!(
        Error::Database(sqlx::Error::PoolTimedOut).fault(),
        "database"
    );
}
