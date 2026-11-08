//! K2 "Roles", DB-005: the runtime role writes and reads metrics and manages partitions only
//! through the owner's functions.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};

use super::database::{PVE_01, database_now, day_at, device, rows, write};
use crate::metrics::{
    MetricKind, MetricRange, Retention, create_partitions_ahead, drop_expired_partitions, gpus,
    readings, series,
};

/// A pool whose every connection runs as `anima_runtime`.
async fn runtime_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE anima_runtime").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

async fn refusal(pool: &PgPool, sql: &str) -> Option<String> {
    let error = sqlx::query(sql).execute(pool).await.err()?;
    error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .map(|code| code.to_string())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_009_runtime_role_writes_reads_and_runs_the_partition_upkeep(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let pve = device(PVE_01);
    let now: DateTime<Utc> = database_now(&pool).await;
    let at = day_at(&pool, 1, "10:00:00").await;

    create_partitions_ahead(&runtime, now).await.unwrap();
    for _ in 0..2 {
        write(&runtime, pve, at, &[(MetricKind::CpuPercent, "", 10.0)]).await;
        write(
            &runtime,
            pve,
            at + TimeDelta::seconds(30),
            &[(MetricKind::CpuPercent, "", 30.0)],
        )
        .await;
    }
    let read = readings(&runtime, &[pve], at + TimeDelta::seconds(40))
        .await
        .unwrap();
    let charts = series(&runtime, pve, MetricRange::Day, at + TimeDelta::minutes(5))
        .await
        .unwrap();
    let found_gpus = gpus(&runtime, &[pve]).await.unwrap();
    let trimmed = drop_expired_partitions(&runtime, now, Retention::default())
        .await
        .unwrap();

    assert_eq!(read[&pve].cpu_percent, Some(30.0));
    assert_eq!(charts[0].points.len(), 1);
    assert!(found_gpus.is_empty());
    assert_eq!(trimmed.partitions.len(), 0);
    assert_eq!(rows(&pool, "samples").await, 2);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_009_runtime_role_cannot_create_or_drop_a_partition_directly(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let at = day_at(&pool, 1, "10:00:00").await;
    write(
        &runtime,
        device(PVE_01),
        at,
        &[(MetricKind::CpuPercent, "", 1.0)],
    )
    .await;
    let existing = format!("metrics.samples_{}", at.format("%Y%m%d"));

    let statements = [
        "create table metrics.samples_20990101 partition of metrics.samples \
         for values from ('2099-01-01T00:00:00Z') to ('2099-01-02T00:00:00Z')"
            .to_owned(),
        "drop table metrics.samples".to_owned(),
        format!("drop table {existing}"),
        format!("alter table metrics.samples detach partition {existing}"),
        "truncate metrics.samples".to_owned(),
        "create table metrics.extra (id int)".to_owned(),
    ];

    for sql in statements {
        assert_eq!(
            refusal(&runtime, &sql).await.as_deref(),
            Some("42501"),
            "{sql}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn db_005_runtime_role_cannot_drop_a_day_inside_the_one_day_floor_through_the_function(
    pool: PgPool,
) {
    let runtime = runtime_pool(&pool).await;
    let at = day_at(&pool, 3, "10:00:00").await;
    write(
        &runtime,
        device(PVE_01),
        at,
        &[(MetricKind::CpuPercent, "", 1.0)],
    )
    .await;

    for days in [0, -1, -29] {
        let sql = format!(
            "select metrics.drop_day_partitions_before(current_date - ({days}), now() + interval '90 days')"
        );
        assert_eq!(
            refusal(&runtime, &sql).await.as_deref(),
            Some("42501"),
            "{days} days"
        );
    }
    let far = "select metrics.ensure_day_partition(now() + interval '9 days')";
    assert_eq!(refusal(&runtime, far).await.as_deref(), Some("22023"));
    assert_eq!(rows(&pool, "samples").await, 1);

    let allowed = "select metrics.drop_day_partitions_before(current_date - 1, now())";
    assert_eq!(refusal(&runtime, allowed).await, None);
    assert_eq!(rows(&pool, "samples").await, 0);
}
