//! Heartbeat retention drops whole day partitions older than 30 days, and the database
//! refuses to drop younger ones (DB-005, DB-009).

use chrono::{DateTime, Days, NaiveDate, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

use super::{as_runtime, code_of, create_day_as_owner, ensure, execute, table_exists};

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
    let mut conn = pool.acquire().await.unwrap().detach();

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
    let ago = |days: i64| (now - TimeDelta::days(days)).date_naive();
    let name = |days: i64| format!("heartbeats_{}", ago(days).format("%Y%m%d"));
    let mut conn = pool.acquire().await.unwrap().detach();
    for days in [60, 45, 40] {
        create_day_as_owner(&mut conn, ago(days)).await;
    }
    for days in [30, 5] {
        ensure(&mut conn, now - TimeDelta::days(days))
            .await
            .unwrap();
    }
    // Tables the function must leave alone: a partition whose name is not a day; a partition
    // with a day name in another schema; and a plain table of that name in `checks`.
    let (stray, stray_day) = (name(50), ago(50));
    for sql in [
        "create table checks.heartbeats_archive partition of checks.heartbeats \
         for values from ('2000-01-01T00:00:00Z') to ('2000-01-02T00:00:00Z')"
            .to_owned(),
        "create schema elsewhere".to_owned(),
        format!(
            "create table elsewhere.{stray} partition of checks.heartbeats \
             for values from ('{stray_day}T00:00:00Z') to ('{}T00:00:00Z')",
            stray_day.succ_opt().unwrap()
        ),
        format!("create table checks.{stray} (id int)"),
    ] {
        execute(&mut conn, &sql).await;
    }
    let cutoff = ago(40);

    let dropped = as_runtime(&mut conn, async |conn| {
        drop_before(conn, Some(cutoff), now).await
    })
    .await;

    assert_eq!(dropped, Ok(vec![name(60), name(45)]));
    let kept = [name(40), name(30), name(5), "heartbeats_archive".to_owned()];
    let kept = kept.map(|table| format!("checks.{table}"));
    for kept in kept
        .iter()
        .chain([&format!("checks.{stray}"), &format!("elsewhere.{stray}")])
    {
        assert!(table_exists(&mut conn, kept).await, "{kept}");
    }
    for gone in [name(60), name(45)] {
        let gone = format!("checks.{gone}");
        assert!(!table_exists(&mut conn, &gone).await, "{gone}");
    }
}
