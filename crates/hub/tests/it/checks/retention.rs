//! Heartbeat retention drops whole day partitions older than 30 days, and the database
//! refuses to drop younger ones (DB-005, DB-009).

use chrono::{DateTime, Days, NaiveDate, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

use super::{as_runtime, code_of, ensure, execute, table_exists};

/// The test database's clock: the drop function bounds its cutoff by it.
async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn drop_before(
    conn: &mut PgConnection,
    cutoff: Option<NaiveDate>,
    instant: DateTime<Utc>,
) -> Result<Vec<String>, String> {
    sqlx::query_scalar("select checks.drop_day_partitions_before($1, $2)")
        .bind(cutoff)
        .bind(instant)
        .fetch_all(conn)
        .await
        .map_err(|error| code_of(&error))
}

#[sqlx::test(migrations = "./migrations")]
async fn db_009_drop_refuses_a_cutoff_inside_30_days_even_with_a_later_instant(pool: PgPool) {
    let now = database_now(&pool).await;
    let today = now.date_naive();
    let later = now + TimeDelta::days(3 * 366);
    let refused = [
        (None, now),
        (Some(today - Days::new(29)), now),
        (Some(today), now),
        (Some(today - Days::new(10)), later),
        (Some(today - Days::new(29)), later),
    ];
    let accepted = [
        (Some(today - Days::new(30)), now),
        (Some(today - Days::new(30)), later),
        (Some(today - Days::new(400)), now),
    ];
    let mut conn = pool.acquire().await.unwrap();

    let outcomes = as_runtime(&mut conn, async |conn| {
        let mut outcomes = Vec::new();
        for (cutoff, instant) in refused.into_iter().chain(accepted) {
            outcomes.push(((cutoff, instant), drop_before(conn, cutoff, instant).await));
        }
        outcomes
    })
    .await;

    let (refusals, successes) = outcomes.split_at(refused.len());
    for (call, outcome) in refusals {
        assert_eq!(outcome, &Err("42501".to_owned()), "{call:?}");
    }
    for (call, outcome) in successes {
        assert_eq!(outcome, &Ok(Vec::new()), "{call:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn db_005_drop_removes_only_heartbeat_days_before_the_cutoff(pool: PgPool) {
    let now = database_now(&pool).await;
    let ago = |days: i64| now - TimeDelta::days(days);
    let name = |days: i64| format!("heartbeats_{}", ago(days).format("%Y%m%d"));
    let mut conn = pool.acquire().await.unwrap();
    for days in [60, 45, 40, 31, 5] {
        ensure(&mut conn, ago(days)).await.unwrap();
    }
    // Tables the function must leave alone: a partition whose name is not a day, and a
    // table with a day name that is not a heartbeat partition.
    execute(
        &mut conn,
        "create table checks.heartbeats_archive partition of checks.heartbeats \
         for values from ('2000-01-01T00:00:00Z') to ('2000-01-02T00:00:00Z')",
    )
    .await;
    execute(
        &mut conn,
        "create table checks.heartbeats_19990101 (id int)",
    )
    .await;
    let cutoff = ago(40).date_naive();

    let dropped = as_runtime(&mut conn, async |conn| {
        drop_before(conn, Some(cutoff), now).await
    })
    .await;

    assert_eq!(dropped, Ok(vec![name(60), name(45)]));
    for kept in [
        name(40),
        name(31),
        name(5),
        "heartbeats_archive".to_owned(),
        "heartbeats_19990101".to_owned(),
    ] {
        let kept = format!("checks.{kept}");
        assert!(table_exists(&mut conn, &kept).await, "{kept}");
    }
    for gone in [name(60), name(45)] {
        let gone = format!("checks.{gone}");
        assert!(!table_exists(&mut conn, &gone).await, "{gone}");
    }
}
