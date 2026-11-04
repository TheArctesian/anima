//! `uptimes`: from the aggregate tables only, over the window that ends now (CHK-011, D-4).

use std::collections::HashMap;

use anima_hub::{
    checks::{self, Definition, UptimeWindow, config::CheckConfig},
    ids::{CheckId, DeviceId},
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{DOCKER_HOST, PVE_01, PVE_01_PING, PVE_01_SSH, check, create, device, new_check};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

/// Beats in one bucket: (up, down, degraded, maintenance, unknown).
type Counts = (i32, i32, i32, i32, i32);

async fn aggregate(
    pool: &PgPool,
    table: &str,
    (id, bucket): (CheckId, DateTime<Utc>),
    counts: Counts,
) {
    let (up, down, degraded, maintenance, unknown) = counts;
    sqlx::query(&format!(
        "INSERT INTO checks.{table} (check_id, bucket, up_count, down_count, degraded_count, \
                maintenance_count, unknown_count) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    ))
    .bind(id)
    .bind(bucket)
    .bind(up)
    .bind(down)
    .bind(degraded)
    .bind(maintenance)
    .bind(unknown)
    .execute(pool)
    .await
    .unwrap();
}

async fn uptime(pool: &PgPool, window: UptimeWindow) -> HashMap<DeviceId, f64> {
    let devices = [device(PVE_01), device(DOCKER_HOST)];
    checks::uptimes(pool, &devices, window, now())
        .await
        .unwrap()
}

fn only_pve(ratio: f64) -> HashMap<DeviceId, f64> {
    HashMap::from([(device(PVE_01), ratio)])
}

fixture_test! {
    async fn chk_011_uptime_sums_the_device_checks_and_counts_degraded_as_up(pool: PgPool) {
        let hour_ago = now() - TimeDelta::hours(1);
        aggregate(&pool, "uptime_minute", (check(PVE_01_PING), hour_ago), (2, 0, 1, 0, 0)).await;
        aggregate(&pool, "uptime_minute", (check(PVE_01_SSH), hour_ago), (0, 1, 0, 0, 0)).await;
        assert_eq!(uptime(&pool, UptimeWindow::Day).await, only_pve(0.75));
    }

    async fn chk_011_uptime_never_reads_raw_heartbeats(pool: PgPool) {
        let beat_at: DateTime<Utc> = sqlx::query_scalar("SELECT now()").fetch_one(&pool).await.unwrap();
        sqlx::query("SELECT checks.ensure_day_partition($1)").bind(beat_at).execute(&pool).await.unwrap();
        for second in 0..20 {
            sqlx::query(
                "INSERT INTO checks.heartbeats (check_id, at, status) VALUES ($1, $2, 'down')",
            )
            .bind(check(PVE_01_PING))
            .bind(beat_at - TimeDelta::seconds(second))
            .execute(&pool)
            .await
            .unwrap();
        }
        let devices = [device(PVE_01)];
        let from_beats = checks::uptimes(&pool, &devices, UptimeWindow::Day, beat_at).await.unwrap();
        assert!(from_beats.is_empty(), "{from_beats:?}");

        aggregate(&pool, "uptime_minute", (check(PVE_01_PING), beat_at - TimeDelta::hours(1)), (4, 0, 0, 0, 0)).await;
        let from_aggregates = checks::uptimes(&pool, &devices, UptimeWindow::Day, beat_at).await.unwrap();
        assert_eq!(from_aggregates, only_pve(1.0));
    }

    async fn chk_011_maintenance_and_unknown_beats_are_not_counted(pool: PgPool) {
        let hour_ago = now() - TimeDelta::hours(1);
        aggregate(&pool, "uptime_minute", (check(PVE_01_PING), hour_ago), (1, 0, 0, 9, 9)).await;
        aggregate(&pool, "uptime_minute", (check(PVE_01_SSH), hour_ago), (0, 0, 0, 5, 5)).await;
        assert_eq!(uptime(&pool, UptimeWindow::Day).await, only_pve(1.0));
    }

    async fn chk_011_a_device_with_no_counted_beats_has_no_uptime(pool: PgPool) {
        let hour_ago = now() - TimeDelta::hours(1);
        aggregate(&pool, "uptime_minute", (check(PVE_01_PING), hour_ago), (0, 0, 0, 3, 3)).await;
        assert!(uptime(&pool, UptimeWindow::Day).await.is_empty());
    }

    async fn chk_011_the_window_holds_buckets_that_start_in_it(pool: PgPool) {
        let start = now() - TimeDelta::days(1);
        let ping = check(PVE_01_PING);
        aggregate(&pool, "uptime_minute", (ping, start), (1, 0, 0, 0, 0)).await;
        aggregate(&pool, "uptime_minute", (ping, start - TimeDelta::minutes(1)), (0, 50, 0, 0, 0)).await;
        aggregate(&pool, "uptime_minute", (ping, now()), (0, 50, 0, 0, 0)).await;
        assert_eq!(uptime(&pool, UptimeWindow::Day).await, only_pve(1.0));
    }

    async fn chk_011_each_window_reads_its_own_table(pool: PgPool) {
        let ping = check(PVE_01_PING);
        aggregate(&pool, "uptime_minute", (ping, now() - TimeDelta::hours(2)), (1, 1, 0, 0, 0)).await;
        aggregate(&pool, "uptime_hour", (ping, now() - TimeDelta::days(20)), (3, 1, 0, 0, 0)).await;
        aggregate(&pool, "uptime_day", (ping, now() - TimeDelta::days(200)), (1, 4, 0, 0, 0)).await;
        assert_eq!(uptime(&pool, UptimeWindow::Day).await, only_pve(0.5));
        assert!(uptime(&pool, UptimeWindow::Week).await.is_empty());
        assert_eq!(uptime(&pool, UptimeWindow::Month).await, only_pve(0.75));
        assert_eq!(uptime(&pool, UptimeWindow::Year).await, only_pve(0.2));
    }

    async fn chk_011_group_checks_add_nothing_to_uptime(pool: PgPool) {
        let group = create(&pool, new_check(PVE_01, "Group", Definition::Config(CheckConfig::Group))).await;
        let hour_ago = now() - TimeDelta::hours(1);
        aggregate(&pool, "uptime_minute", (group, hour_ago), (0, 7, 0, 0, 0)).await;
        aggregate(&pool, "uptime_minute", (check(PVE_01_PING), hour_ago), (1, 0, 0, 0, 0)).await;
        assert_eq!(uptime(&pool, UptimeWindow::Day).await, only_pve(1.0));
    }
}
