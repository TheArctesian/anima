//! Dropping old day partitions, and the database's refusal to drop inside retention
//! (DB-005, LOG-006, R-016).

use chrono::{DateTime, Days, NaiveDate, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

use super::{partitions::table_exists, pool_as, privileges::as_runtime};
use crate::events::{create_partitions_ahead, drop_expired_partitions};

/// The test database's clock: the drop function bounds its cutoff by it.
async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Calls the drop function directly and returns its SQLSTATE, or `None` if it succeeded.
async fn drop_refusal(
    conn: &mut PgConnection,
    (table, cutoff, instant): (&str, NaiveDate, DateTime<Utc>),
) -> Option<String> {
    let error = sqlx::query("select events.drop_day_partitions_before($1, $2, $3)")
        .bind(table)
        .bind(cutoff)
        .bind(instant)
        .execute(conn)
        .await
        .err()?;
    let code = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    Some(code.map_or_else(|| error.to_string(), Into::into))
}

#[sqlx::test(migrations = "./migrations")]
async fn log_006_drop_function_refuses_a_cutoff_inside_retention_even_with_a_later_now(
    pool: PgPool,
) {
    let now = database_now(&pool).await;
    let today = now.date_naive();
    let later = now + TimeDelta::days(3 * 366);
    let attempts = [
        (("audit_log", today - Days::new(30), later), Some("42501")),
        (("audit_log", today - Days::new(300), later), Some("42501")),
        (("event_log", today - Days::new(10), later), Some("42501")),
        (("event_log", today - Days::new(60), now), Some("42501")),
        (("audit_log", today - Days::new(400), now), None),
        (("event_log", today - Days::new(100), now), None),
        (("access_log", today - Days::new(800), now), Some("22023")),
    ];
    let mut conn = pool.acquire().await.unwrap().detach();

    let outcomes = as_runtime(&mut conn, async |conn| {
        let mut outcomes = Vec::new();
        for (call, _) in attempts {
            outcomes.push(drop_refusal(conn, call).await);
        }
        outcomes
    })
    .await;

    for ((call, expected), outcome) in attempts.iter().zip(outcomes) {
        assert_eq!(outcome.as_deref(), *expected, "{call:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn db_005_drop_expired_partitions_removes_old_partitions_and_keeps_newer(pool: PgPool) {
    let now = database_now(&pool).await;
    let ago = |days: i64| now - TimeDelta::days(days);
    let name = |table: &str, days: i64| format!("{table}_{}", ago(days).format("%Y%m%d"));
    for days in [500, 200, 120, 60] {
        create_partitions_ahead(&pool, ago(days), 0).await.unwrap();
    }
    let runtime = pool_as(&pool, "anima_runtime").await;

    let mut dropped = drop_expired_partitions(&runtime, now).await.unwrap();

    runtime.close().await;
    dropped.sort();
    let mut expected = vec![
        name("event_log", 500),
        name("event_log", 200),
        name("event_log", 120),
        name("audit_log", 500),
    ];
    expected.sort();
    assert_eq!(dropped, expected);
    let mut conn = pool.acquire().await.unwrap();
    for (table, days) in [
        ("event_log", 60),
        ("audit_log", 200),
        ("audit_log", 120),
        ("audit_log", 60),
    ] {
        let kept = format!("events.{}", name(table, days));
        assert!(table_exists(&mut conn, &kept).await, "{kept}");
    }
}
