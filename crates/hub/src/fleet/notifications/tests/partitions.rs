//! The delivery partitions: created ahead, dropped past retention (DB-004, DB-005, D-25).

use chrono::{Days, TimeDelta};
use sqlx::PgPool;

use super::{now, refusal, runtime_pool};
use crate::fleet::notifications::{create_partitions_ahead, drop_expired_deliveries};

async fn partitions(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "select c.relname::text from pg_inherits i join pg_class c on c.oid = i.inhrelid \
         where i.inhparent = 'notifications.deliveries'::regclass order by c.relname",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Creates the partition for the UTC day `days_ago` days back, as the owner (a test setup that
/// the 31-day window of the hub's function would refuse).
async fn old_partition(pool: &PgPool, days_ago: i64) -> String {
    let name: String = sqlx::query_scalar(
        "select 'deliveries_' || to_char((now() at time zone 'UTC')::date - $1::int, 'YYYYMMDD')",
    )
    .bind(i32::try_from(days_ago).unwrap())
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "create table notifications.{name} partition of notifications.deliveries \
         for values from (((now() at time zone 'UTC')::date - {days_ago})::timestamp \
         at time zone 'UTC') \
         to (((now() at time zone 'UTC')::date - {days_ago} + 1)::timestamp at time zone 'UTC')"
    ))
    .execute(pool)
    .await
    .unwrap();
    name
}

#[sqlx::test(migrations = "./migrations")]
async fn db_004_the_runtime_role_creates_today_and_seven_days_ahead(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;

    create_partitions_ahead(&runtime, now(&pool).await)
        .await
        .unwrap();
    create_partitions_ahead(&runtime, now(&pool).await)
        .await
        .unwrap();

    assert_eq!(partitions(&pool).await.len(), 8);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_005_the_drop_removes_only_partitions_before_the_cutoff_day(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let oldest = old_partition(&pool, 40).await;
    let old = old_partition(&pool, 31).await;
    let edge = old_partition(&pool, 30).await;
    let recent = old_partition(&pool, 20).await;
    create_partitions_ahead(&runtime, now(&pool).await)
        .await
        .unwrap();

    let dropped = drop_expired_deliveries(&runtime, now(&pool).await, Days::new(30))
        .await
        .unwrap();

    assert_eq!(dropped, [oldest, old]);
    let left = partitions(&pool).await;
    assert!(left.contains(&edge) && left.contains(&recent));
    assert_eq!(left.len(), 2 + 8);
}

#[sqlx::test(migrations = "./migrations")]
async fn db_005_the_database_refuses_to_drop_inside_seven_days_whatever_the_caller_says(
    pool: PgPool,
) {
    let runtime = runtime_pool(&pool).await;
    old_partition(&pool, 5).await;
    let later = now(&pool).await + TimeDelta::days(90);

    let short = drop_expired_deliveries(&runtime, later, Days::new(3)).await;

    assert!(short.is_err());
    let sql =
        "select notifications.drop_day_partitions_before(current_date, now() + interval '90 days')";
    assert_eq!(refusal(&runtime, sql).await.as_deref(), Some("42501"));
    assert_eq!(partitions(&pool).await.len(), 1);
}
