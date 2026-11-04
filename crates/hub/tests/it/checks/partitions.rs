//! Heartbeats live in one partition per UTC day, created by `checks.ensure_day_partition`
//! (DB-004).

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{beat, code_of, database_now, ensure, execute, partition, refusal, table_exists};

/// The database clock moved by `days` (negative is earlier).
async fn days_from_now(pool: &PgPool, days: i64) -> DateTime<Utc> {
    database_now(&mut pool.acquire().await.unwrap()).await + TimeDelta::days(days)
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_a_heartbeat_needs_its_day_partition(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let (check, instant) = (Uuid::now_v7(), days_from_now(&pool, 1).await);

    let missing = beat(&mut conn, check, instant).await.unwrap_err();
    ensure(&mut conn, instant).await.unwrap();
    ensure(&mut conn, instant).await.unwrap();
    let stored = beat(&mut conn, check, instant).await.unwrap();

    assert_eq!(code_of(&missing), "23514");
    assert_eq!(stored, partition(instant));
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_ensure_partition_succeeds_after_a_racing_creator_commits(pool: PgPool) {
    let instant = days_from_now(&pool, 2).await;
    let mut first = pool.begin().await.unwrap();
    ensure(&mut first, instant).await.unwrap();

    let mut second = pool.acquire().await.unwrap();
    let second_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *second)
        .await
        .unwrap();
    let racing = tokio::spawn(async move { ensure(&mut second, instant).await });

    // Wait until the second call is blocked behind the first one's uncommitted partition.
    let mut observer = pool.acquire().await.unwrap();
    let mut waiting = false;
    for _ in 0..500 {
        waiting =
            sqlx::query_scalar("select cardinality(pg_blocking_pids($1)) > 0 from pg_sleep(0.01)")
                .bind(second_pid)
                .fetch_one(&mut *observer)
                .await
                .unwrap();
        if waiting {
            break;
        }
    }
    assert!(waiting, "the second call never waited for the first");
    first.commit().await.unwrap();

    racing.await.unwrap().unwrap();
    assert!(table_exists(&mut observer, &partition(instant)).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_concurrent_ensure_calls_on_a_new_day_all_succeed(pool: PgPool) {
    let instant = days_from_now(&pool, 3).await;
    let mut callers = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let pool = pool.clone();
        callers.spawn(async move {
            let mut tx = pool.begin().await.unwrap();
            ensure(&mut tx, instant).await?;
            tx.commit().await
        });
    }

    for outcome in callers.join_all().await {
        outcome.unwrap();
    }
    let mut conn = pool.acquire().await.unwrap();
    assert!(table_exists(&mut conn, &partition(instant)).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_day_bounds_are_utc_whatever_the_session_time_zone(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let check = Uuid::now_v7();
    let day = days_from_now(&pool, 2).await.date_naive();
    let midnight = day.and_time(NaiveTime::MIN).and_utc();
    // 03:00 UTC is still the previous day in Los Angeles.
    execute(&mut conn, "set time zone 'America/Los_Angeles'").await;
    ensure(&mut conn, midnight + TimeDelta::hours(3))
        .await
        .unwrap();
    execute(&mut conn, "set time zone 'Pacific/Kiritimati'").await;

    let first = beat(&mut conn, check, midnight).await.unwrap();
    let last = beat(
        &mut conn,
        check,
        midnight + TimeDelta::days(1) - TimeDelta::microseconds(1),
    )
    .await
    .unwrap();
    let before = beat(&mut conn, check, midnight - TimeDelta::microseconds(1)).await;
    let after = beat(&mut conn, check, midnight + TimeDelta::days(1)).await;

    assert_eq!(first, partition(midnight));
    assert_eq!(last, partition(midnight));
    assert_eq!(code_of(&before.unwrap_err()), "23514");
    assert_eq!(code_of(&after.unwrap_err()), "23514");
    let previous = partition(midnight - TimeDelta::days(1));
    assert!(!table_exists(&mut conn, &previous).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_ensure_accepts_only_instants_from_31_days_ago_to_7_days_ahead(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let mut outcomes = Vec::new();
    for (days, accepted) in [(-32, false), (-30, true), (6, true), (8, false)] {
        let instant = days_from_now(&pool, days).await;
        let outcome = ensure(&mut conn, instant).await.err().map(|e| code_of(&e));
        let exists = table_exists(&mut conn, &partition(instant)).await;
        outcomes.push((days, accepted, outcome, exists));
    }

    for (days, accepted, outcome, exists) in outcomes {
        let expected = (!accepted).then(|| "22023".to_owned());
        assert_eq!(outcome, expected, "{days} days");
        assert_eq!(exists, accepted, "{days} days");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_ensure_refuses_an_instant_without_a_named_day(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let instants = [
        "null",
        "'infinity'",
        "'-infinity'",
        "'10000-01-01T00:00:00Z'",
        "'0500-06-01 00:00:00+00 BC'",
    ];
    let mut outcomes = Vec::new();
    for instant in instants {
        let sql = format!("select checks.ensure_day_partition({instant})");
        outcomes.push((instant, refusal(&mut conn, &sql).await));
    }
    let children: i64 = sqlx::query_scalar(
        "select count(*) from pg_catalog.pg_inherits \
         where inhparent = 'checks.heartbeats'::regclass",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();

    for (instant, outcome) in outcomes {
        assert_eq!(outcome.as_deref(), Some("22023"), "{instant}");
    }
    assert_eq!(children, 0);
}
