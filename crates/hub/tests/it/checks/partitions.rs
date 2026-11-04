//! Heartbeats live in one partition per UTC day, created by `checks.ensure_day_partition`
//! (DB-004).

use sqlx::PgPool;
use uuid::Uuid;

use super::{at, beat, code_of, ensure, execute, refusal, table_exists};

#[sqlx::test(migrations = "./migrations")]
async fn db_004_a_heartbeat_needs_its_day_partition(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let (check, instant) = (Uuid::now_v7(), at("2030-01-15T12:30:00Z"));

    let missing = beat(&mut conn, check, instant).await.unwrap_err();
    ensure(&mut conn, instant).await.unwrap();
    ensure(&mut conn, instant).await.unwrap();
    let partition = beat(&mut conn, check, instant).await.unwrap();

    assert_eq!(code_of(&missing), "23514");
    assert_eq!(partition, "checks.heartbeats_20300115");
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_ensure_partition_succeeds_after_a_racing_creator_commits(pool: PgPool) {
    let instant = at("2029-02-28T08:00:00Z");
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
    assert!(table_exists(&mut observer, "checks.heartbeats_20290228").await);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_concurrent_ensure_calls_on_a_new_day_all_succeed(pool: PgPool) {
    let instant = at("2029-03-01T00:00:00Z");
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
    assert!(table_exists(&mut conn, "checks.heartbeats_20290301").await);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_day_bounds_are_utc_whatever_the_session_time_zone(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let check = Uuid::now_v7();
    // 03:00 UTC is still the previous day in Los Angeles.
    execute(&mut conn, "set time zone 'America/Los_Angeles'").await;
    ensure(&mut conn, at("2030-01-01T03:00:00Z")).await.unwrap();
    execute(&mut conn, "set time zone 'Pacific/Kiritimati'").await;

    let first = beat(&mut conn, check, at("2030-01-01T00:00:00Z"))
        .await
        .unwrap();
    let last: Result<String, _> = sqlx::query_scalar(
        "insert into checks.heartbeats (check_id, at, status) \
         values ($1, '2030-01-01T23:59:59.999999Z', 'up') returning tableoid::regclass::text",
    )
    .bind(check)
    .fetch_one(&mut *conn)
    .await;
    let before = beat(&mut conn, check, at("2029-12-31T23:59:00Z")).await;
    let after = beat(&mut conn, check, at("2030-01-02T00:00:00Z")).await;

    assert_eq!(first, "checks.heartbeats_20300101");
    assert_eq!(last.unwrap(), "checks.heartbeats_20300101");
    assert_eq!(code_of(&before.unwrap_err()), "23514");
    assert_eq!(code_of(&after.unwrap_err()), "23514");
    assert!(!table_exists(&mut conn, "checks.heartbeats_20291231").await);
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
