//! `statuses`, `device_status` and `resolved_ips` against stored state, the manual flag and
//! maintenance windows (CHK-004, CHK-006).

use std::collections::HashMap;

use anima_hub::{
    checks::{self, Error, Missing},
    ids::DeviceId,
    status::Status::{self, Degraded, Down, Maintenance, Unknown, Up},
};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    DOCKER_HOST, JELLYFIN, PVE_01, PVE_01_PING, PVE_01_SSH, TRUENAS, beat, check, database_now,
    device, record, set_status,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks", "maintenance"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

/// The "Ping" check of `jellyfin` and of `truenas` in the fixtures.
const JELLYFIN_PING: &str = "0190a000-0000-7000-8003-00000000000a";
const TRUENAS_PING: &str = "0190a000-0000-7000-8003-00000000000c";
const TRUENAS_SSH: &str = "0190a000-0000-7000-8003-00000000000d";

/// 22:00 NZDT on 4 November: no fixture window runs.
fn quiet() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

/// 23:45 NZDT on 4 November: the "Overnight backup" window (tag `storage`) runs.
fn during_backup() -> DateTime<Utc> {
    "2026-11-04T10:45:00Z".parse().unwrap()
}

async fn status_of(pool: &PgPool, text: &str, now: DateTime<Utc>) -> Status {
    let id = device(text);
    checks::statuses(pool, &[id], now).await.unwrap()[&id]
}

/// A manual window, switched on, with one target.
async fn manual_window(pool: &PgPool, target_type: &str, target: &str) {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO maintenance.windows (id, name, kind, timezone, rule, active_manual) \
         VALUES ($1, $2, 'manual', 'UTC', '{\"kind\": \"manual\"}', true)",
    )
    .bind(id)
    .bind(format!("window {id}"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO maintenance.window_targets (window_id, target_type, target) \
         VALUES ($1, $2, $3)",
    )
    .bind(id)
    .bind(target_type)
    .bind(target)
    .execute(pool)
    .await
    .unwrap();
}

/// Every fixture check Up, so one changed check decides its device.
async fn all_up(pool: &PgPool) {
    sqlx::query("UPDATE checks.checks SET status = 'up'")
        .execute(pool)
        .await
        .unwrap();
}

fixture_test! {
    async fn chk_004_each_device_shows_the_worst_of_its_checks(pool: PgPool) {
        set_status(&pool, PVE_01_PING, "up").await;
        set_status(&pool, PVE_01_SSH, "degraded").await;
        sqlx::query("UPDATE checks.checks SET status = 'up' WHERE device_id = $1::text::uuid")
            .bind(DOCKER_HOST)
            .execute(&pool)
            .await
            .unwrap();
        let requested = [device(PVE_01), device(DOCKER_HOST), device(TRUENAS)];
        let shown = checks::statuses(&pool, &requested, quiet()).await.unwrap();
        let expected = HashMap::from([
            (device(PVE_01), Degraded),
            (device(DOCKER_HOST), Up),
            (device(TRUENAS), Unknown),
        ]);
        assert_eq!(shown, expected);
    }

    async fn chk_004_a_device_without_checks_is_unknown(pool: PgPool) {
        let stranger = DeviceId::from(Uuid::now_v7());
        let shown = checks::statuses(&pool, &[stranger], quiet()).await.unwrap();
        assert_eq!(shown, HashMap::from([(stranger, Unknown)]));
    }

    async fn chk_005_a_paused_down_check_does_not_count(pool: PgPool) {
        all_up(&pool).await;
        set_status(&pool, PVE_01_PING, "up").await;
        set_status(&pool, PVE_01_SSH, "down").await;
        assert_eq!(status_of(&pool, PVE_01, quiet()).await, Down);
        sqlx::query("UPDATE checks.checks SET paused = true WHERE id = $1::text::uuid")
            .bind(PVE_01_SSH)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(status_of(&pool, PVE_01, quiet()).await, Up);
    }

    async fn chk_006_the_manual_flag_turns_down_into_maintenance(pool: PgPool) {
        all_up(&pool).await;
        set_status(&pool, JELLYFIN_PING, "down").await;
        assert_eq!(status_of(&pool, JELLYFIN, quiet()).await, Maintenance);
        set_status(&pool, JELLYFIN_PING, "degraded").await;
        assert_eq!(status_of(&pool, JELLYFIN, quiet()).await, Degraded);
    }

    async fn chk_006_a_tag_window_covers_the_device_only_while_it_runs(pool: PgPool) {
        set_status(&pool, TRUENAS_PING, "down").await;
        set_status(&pool, TRUENAS_SSH, "up").await;
        assert_eq!(status_of(&pool, TRUENAS, quiet()).await, Down);
        assert_eq!(status_of(&pool, TRUENAS, during_backup()).await, Maintenance);
        // pve-01 has no `storage` tag.
        set_status(&pool, PVE_01_PING, "down").await;
        assert_eq!(status_of(&pool, PVE_01, during_backup()).await, Down);
    }

    async fn chk_006_a_device_window_covers_every_check_of_the_device(pool: PgPool) {
        all_up(&pool).await;
        set_status(&pool, PVE_01_PING, "down").await;
        set_status(&pool, PVE_01_SSH, "down").await;
        manual_window(&pool, "device", PVE_01).await;
        assert_eq!(status_of(&pool, PVE_01, quiet()).await, Maintenance);
        set_status(&pool, TRUENAS_PING, "down").await;
        assert_eq!(status_of(&pool, TRUENAS, quiet()).await, Down);
    }

    async fn chk_006_a_check_window_covers_only_that_check(pool: PgPool) {
        all_up(&pool).await;
        set_status(&pool, PVE_01_PING, "up").await;
        set_status(&pool, PVE_01_SSH, "down").await;
        manual_window(&pool, "check", PVE_01_SSH).await;
        assert_eq!(status_of(&pool, PVE_01, quiet()).await, Maintenance);
        set_status(&pool, PVE_01_PING, "down").await;
        assert_eq!(status_of(&pool, PVE_01, quiet()).await, Down);
    }

    async fn chk_004_device_status_matches_statuses_and_knows_missing_devices(pool: PgPool) {
        all_up(&pool).await;
        set_status(&pool, JELLYFIN_PING, "down").await;
        let one = checks::device_status(&pool, device(JELLYFIN), quiet()).await.unwrap();
        assert_eq!(one, status_of(&pool, JELLYFIN, quiet()).await);
        assert_eq!(one, Maintenance);
        let stranger = DeviceId::from(Uuid::now_v7());
        let error = checks::device_status(&pool, stranger, quiet()).await.unwrap_err();
        assert!(matches!(error, Error::NotFound(Missing::Device(id)) if id == stranger), "{error:?}");
    }

    async fn resolved_ips_take_the_check_with_the_latest_beat(pool: PgPool) {
        let at = database_now(&pool).await;
        let mut ping = beat(at, Status::Up, Some(3));
        ping.resolved_ip = Some("10.10.0.2".parse().unwrap());
        record(&pool, check(PVE_01_PING), &ping).await;
        let mut ssh = beat(at + TimeDelta::seconds(5), Status::Up, Some(3));
        ssh.resolved_ip = Some("10.10.0.3".parse().unwrap());
        record(&pool, check(PVE_01_SSH), &ssh).await;
        // A later beat with no address does not hide the addresses before it.
        record(&pool, check(TRUENAS_PING), &beat(at, Status::Down, None)).await;

        let requested = [device(PVE_01), device(TRUENAS)];
        let ips = checks::resolved_ips(&pool, &requested).await.unwrap();
        assert_eq!(ips, HashMap::from([(device(PVE_01), ssh.resolved_ip.unwrap())]));

        let mut later = beat(at + TimeDelta::seconds(9), Status::Up, Some(3));
        later.resolved_ip = Some("10.10.0.4".parse().unwrap());
        record(&pool, check(PVE_01_PING), &later).await;
        let ips = checks::resolved_ips(&pool, &requested).await.unwrap();
        assert_eq!(ips[&device(PVE_01)], later.resolved_ip.unwrap());
    }
}
