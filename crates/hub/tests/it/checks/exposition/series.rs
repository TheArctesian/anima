//! The K7 series and their values: status, up, response time, uptime windows, certificate
//! days and device status, each as the API shows them (INT-001, CHK-004, CHK-006, CHK-011).

use anima_hub::{checks, ids::DeviceId, status::Status};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::{
    DOCKER_HOST, DOCKER_PING, HA_WEB, JELLYFIN, JELLYFIN_PING, OLLAMA, OLLAMA_PING, OLLAMA_SSH,
    PVE_01, PVE_01_PING, PVE_01_SSH, PVE_01_WEB, body, database_now, of_check, record, rig, scrape,
    set_statuses, value,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices", "checks"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

async fn pause(pool: &PgPool, check: &str) {
    sqlx::query("UPDATE checks.checks SET paused = true WHERE id = $1::uuid")
        .bind(check)
        .execute(pool)
        .await
        .unwrap();
}

/// The K7 value of a status (0 Down, 1 Up, 2 Degraded, 3 Maintenance, 4 Unknown).
const fn k7(status: Status) -> &'static str {
    match status {
        Status::Down => "0",
        Status::Up => "1",
        Status::Degraded => "2",
        Status::Maintenance => "3",
        Status::Unknown => "4",
    }
}

async fn aggregate(pool: &PgPool, table: &str, bucket: DateTime<Utc>, (up, down): (i32, i32)) {
    sqlx::query(&format!(
        "INSERT INTO checks.{table} (check_id, bucket, up_count, down_count, degraded_count, \
                maintenance_count, unknown_count) VALUES ($1::uuid, $2, $3, $4, 0, 0, 0)"
    ))
    .bind(PVE_01_PING)
    .bind(bucket)
    .bind(up)
    .bind(down)
    .execute(pool)
    .await
    .unwrap();
}

fixture_test! {
    async fn int_001_status_and_up_follow_the_api_for_up_down_degraded_paused_and_maintenance(
        pool: PgPool
    ) {
        let now = database_now(&pool).await;
        set_statuses(
            &pool,
            "up",
            &[(PVE_01_PING, "down"), (DOCKER_PING, "degraded"), (OLLAMA_PING, "down"),
              (JELLYFIN_PING, "down")],
        )
        .await;
        pause(&pool, OLLAMA_PING).await;
        let (router, _) = rig(&pool, now);

        let scraped = scrape(&router).await;

        assert_eq!(scraped.status, StatusCode::OK);
        assert_eq!(
            scraped.content_type.as_deref(),
            Some("text/plain; version=0.0.4; charset=utf-8")
        );
        let text = scraped.body;
        let expected = [
            (PVE_01_PING, "0", "0"),
            (PVE_01_SSH, "1", "1"),
            (DOCKER_PING, "2", "1"),
            (OLLAMA_PING, "4", "0"),
            (OLLAMA_SSH, "1", "1"),
            (JELLYFIN_PING, "3", "0"),
        ];
        for (check, status, up) in expected {
            assert_eq!(of_check(&text, "anima_check_status", check).as_deref(), Some(status));
            assert_eq!(of_check(&text, "anima_check_up", check).as_deref(), Some(up));
        }
        let devices: Vec<DeviceId> = [PVE_01, DOCKER_HOST, OLLAMA, JELLYFIN]
            .map(|id| id.parse().unwrap())
            .to_vec();
        let api = checks::statuses(&pool, &devices, now).await.unwrap();
        for device in devices {
            let id = device.to_string();
            let shown = value(&text, "anima_device_status", &[("device_id", &id)]);
            assert_eq!(shown.as_deref(), Some(k7(api[&device])), "{id}");
        }
        assert_eq!(
            [PVE_01, DOCKER_HOST, OLLAMA, JELLYFIN]
                .map(|id| value(&text, "anima_device_status", &[("device_id", id)]).unwrap()),
            ["0", "2", "1", "3"]
        );
    }

    async fn int_001_response_time_is_the_last_heartbeat_and_absent_without_one(pool: PgPool) {
        let now = database_now(&pool).await;
        record(&pool, PVE_01_SSH, now - TimeDelta::seconds(60), Some(400)).await;
        record(&pool, PVE_01_SSH, now - TimeDelta::seconds(30), Some(25)).await;
        record(&pool, DOCKER_PING, now - TimeDelta::seconds(30), None).await;
        let (router, _) = rig(&pool, now);

        let text = body(&router).await;

        let family = "anima_check_response_time_seconds";
        assert_eq!(of_check(&text, family, PVE_01_SSH).as_deref(), Some("0.025"));
        assert_eq!(of_check(&text, family, DOCKER_PING), None);
        assert_eq!(of_check(&text, family, PVE_01_PING), None);
        assert_eq!(text.lines().filter(|l| l.starts_with(family)).count(), 1);
    }

    async fn chk_011_uptime_ratio_has_the_three_windows_from_the_aggregates(pool: PgPool) {
        let now = database_now(&pool).await;
        let hour = now.duration_trunc(TimeDelta::hours(1)).unwrap();
        let minute = now.duration_trunc(TimeDelta::minutes(1)).unwrap();
        aggregate(&pool, "uptime_minute", minute - TimeDelta::hours(1), (3, 1)).await;
        aggregate(&pool, "uptime_hour", hour - TimeDelta::hours(2), (1, 0)).await;
        aggregate(&pool, "uptime_hour", hour - TimeDelta::days(20), (0, 1)).await;
        // A raw heartbeat without its aggregates: uptime never reads it (CHK-011).
        sqlx::query("select checks.ensure_day_partition($1)")
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO checks.heartbeats (check_id, at, status) VALUES ($1::uuid, $2, 'down')",
        )
        .bind(PVE_01_PING)
        .bind(now - TimeDelta::seconds(5))
        .execute(&pool)
        .await
        .unwrap();
        let (router, _) = rig(&pool, now);

        let text = body(&router).await;

        let ratio = |window| {
            let labels = [("check_id", PVE_01_PING), ("window", window)];
            value(&text, "anima_check_uptime_ratio", &labels)
        };
        assert_eq!(ratio("1d").as_deref(), Some("0.75"));
        assert_eq!(ratio("7d").as_deref(), Some("1"));
        assert_eq!(ratio("30d").as_deref(), Some("0.5"));
        let windows = text
            .lines()
            .filter(|line| line.starts_with("anima_check_uptime_ratio{"))
            .count();
        assert_eq!(windows, 3, "only the check with counted beats has the series");
    }

    async fn int_001_certificate_days_only_for_checks_with_a_certificate(pool: PgPool) {
        let now = database_now(&pool).await;
        let expiry = now + TimeDelta::days(10) + TimeDelta::hours(1);
        sqlx::query("UPDATE checks.checks SET cert_not_after = $2 WHERE id = ANY($1::uuid[])")
            .bind([PVE_01_WEB, HA_WEB])
            .bind(expiry)
            .execute(&pool)
            .await
            .unwrap();
        let (router, _) = rig(&pool, now);

        let text = body(&router).await;

        let family = "anima_check_certificate_days_left";
        assert_eq!(of_check(&text, family, PVE_01_WEB).as_deref(), Some("10"));
        assert_eq!(of_check(&text, family, HA_WEB), None, "plain HTTP has no certificate");
        assert_eq!(text.lines().filter(|l| l.starts_with(family)).count(), 1);
        assert!(text.contains("# TYPE anima_check_domain_days_left gauge\n"));
        assert!(!text.contains("anima_check_domain_days_left{"));
    }

    async fn int_002_domain_days_left_only_for_domain_expiry_checks_with_an_expiry_date(
        pool: PgPool
    ) {
        const DOMAIN_CHECK: &str = "0190a000-0000-7000-8003-0000000000d1";
        let now = database_now(&pool).await;
        let expiry = now + TimeDelta::days(12) + TimeDelta::hours(1);
        sqlx::query(
            "INSERT INTO checks.checks \
                 (id, device_id, type, name, config, interval_seconds, retries, domain_not_after) \
             VALUES ($1::uuid, $2::uuid, 'domain_expiry', 'Domain', \
                     '{\"domain\":\"example.com\"}', 86400, 0, $3)",
        )
        .bind(DOMAIN_CHECK)
        .bind(PVE_01)
        .bind(expiry)
        .execute(&pool)
        .await
        .unwrap();
        // The schema refuses a date on any other type; lift that to show the type decides.
        sqlx::query("ALTER TABLE checks.checks DROP CONSTRAINT checks_domain_not_after_check")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE checks.checks SET domain_not_after = $2 WHERE id = $1::uuid")
            .bind(PVE_01_WEB)
            .bind(expiry)
            .execute(&pool)
            .await
            .unwrap();
        let (router, _) = rig(&pool, now);

        let text = body(&router).await;

        let family = "anima_check_domain_days_left";
        let labels = [("check_id", DOMAIN_CHECK), ("check_type", "domain_expiry")];
        assert_eq!(value(&text, family, &labels).as_deref(), Some("12"));
        assert_eq!(of_check(&text, family, PVE_01_WEB), None, "only domain checks have days");
        assert_eq!(text.lines().filter(|l| l.starts_with(&format!("{family}{{"))).count(), 1);
    }

    async fn int_001_every_check_and_device_has_a_status_sample(pool: PgPool) {
        let (router, _) = rig(&pool, database_now(&pool).await);

        let text = body(&router).await;

        let count = |family: &str| {
            text.lines().filter(|l| l.starts_with(&format!("{family}{{"))).count()
        };
        assert_eq!(count("anima_check_status"), 28);
        assert_eq!(count("anima_check_up"), 28);
        assert_eq!(count("anima_device_status"), 13);
        // A check with no result yet is Unknown.
        assert_eq!(of_check(&text, "anima_check_status", PVE_01_PING).as_deref(), Some("4"));
    }

    async fn int_001_only_get_is_served(pool: PgPool) {
        let (router, _) = rig(&pool, database_now(&pool).await);

        let request = Request::builder()
            .method(Method::POST)
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
