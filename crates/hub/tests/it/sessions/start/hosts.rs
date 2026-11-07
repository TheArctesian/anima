//! `startSession` and the device's host (GW-011): an IP literal the target rule or the hub's
//! deny list refuses is a `VALIDATION` error on `host`, reported with display problems; a
//! routable literal or a name starts.

use anima_hub::{sessions::Live, targets::AddressPolicy};
use serde_json::json;
use sqlx::PgPool;

use super::{fields, nothing_written, refusal};
use crate::sessions::{
    DOCKER_HOST,
    harness::{error_json, schema_at},
    now,
    starting::{display, ready, start_request},
};

async fn set_host(pool: &PgPool, device: &str, host: &str) {
    sqlx::query("update devices.devices set host = $2 where id = $1::text::uuid")
        .bind(device)
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn gw_011_start_session_refuses_a_host_literal_that_is_not_one_remote_host(pool: PgPool) {
    ready(&pool, &[DOCKER_HOST]).await;
    for host in [
        "127.0.0.1",
        "127.1.2.3",
        "::1",
        "169.254.169.254",
        "fe80::1",
        "224.0.0.251",
        "ff02::1",
        "0.0.0.0",
        "::",
        "255.255.255.255",
        "::ffff:127.0.0.1",
    ] {
        set_host(&pool, DOCKER_HOST, host).await;
        let error = refusal(&pool, DOCKER_HOST, display()).await;
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{host}: {error}");
        assert_eq!(fields(&error), ["host"], "{host}");
    }
    // A literal the target rule allows is refused when the hub's deny list holds it.
    let deny = AddressPolicy::new(vec!["10.10.0.0/24".parse().unwrap()]);
    let denying = schema_at(&pool, now(), &Live::default().denying(deny));
    for host in ["10.10.0.21", "::ffff:10.10.0.21"] {
        set_host(&pool, DOCKER_HOST, host).await;
        let error = error_json(
            &denying
                .execute(start_request(DOCKER_HOST, &display()))
                .await,
        );
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{host}: {error}");
        assert_eq!(fields(&error), ["host"], "{host}");
    }
    nothing_written(&pool).await;

    // A routable literal and a name (checked when the tunnel connects) are accepted.
    let schema = schema_at(&pool, now(), &Live::default());
    for host in ["10.10.0.21", "fd7a:115c:a1e0::21", "docker-host.home.lan"] {
        set_host(&pool, DOCKER_HOST, host).await;
        let response = schema.execute(start_request(DOCKER_HOST, &display())).await;
        assert!(response.errors.is_empty(), "{host}: {:?}", response.errors);
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn k4_1_start_session_reports_display_and_host_problems_together(pool: PgPool) {
    ready(&pool, &[DOCKER_HOST]).await;
    set_host(&pool, DOCKER_HOST, "127.0.0.1").await;
    let error = refusal(&pool, DOCKER_HOST, json!({ "widthPx": 0, "heightPx": 800 })).await;
    assert_eq!(fields(&error), ["display.widthPx", "host"]);
}
