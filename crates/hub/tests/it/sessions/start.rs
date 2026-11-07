//! `startSession`: the row, the audit entry and the ticket, and each refusal (K4 §1, §6,
//! RA-008, LOG-005, API-009).

use anima_hub::{
    sessions::{self, Live, Protocol},
    targets::AddressPolicy,
};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    DOCKER_HOST, PVE_01, add_operator,
    harness::{CLIENT_ADDR, error_json, schema_at},
    now,
    starting::{
        SessionAudit, display, ready, redeem, session_audits, session_count, start_request,
        started, stored,
    },
};

const HOME_ASSISTANT: &str = "0190a000-0000-7000-8002-000000000004";
const NO_SUCH_DEVICE: &str = "0190a000-0000-7000-8002-0000000000ff";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_009_start_session_writes_a_live_row_and_audit_and_returns_a_ticket(pool: PgPool) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    ready(&pool, &[PVE_01]).await;

    let ticket = started(&schema, PVE_01).await;
    let id = ticket["sessionId"].as_str().unwrap();
    let token = ticket["token"].as_str().unwrap();
    assert_eq!(token.len(), 43);
    assert_eq!(
        ticket["tunnelPath"], "/terminal",
        "K4 §1: SSH uses the terminal tunnel"
    );
    let expires_at: DateTime<Utc> = ticket["expiresAt"].as_str().unwrap().parse().unwrap();
    assert_eq!(expires_at, now() + TimeDelta::seconds(30));

    let row = stored(&pool, id).await;
    assert_eq!(
        (
            row.device_name.as_str(),
            row.operator_name.as_str(),
            row.client.as_str()
        ),
        ("pve-01", "operator", CLIENT_ADDR)
    );
    assert_eq!(
        (row.started_at, row.ended_at, row.end_reason),
        (now(), None, None)
    );

    assert_eq!(
        session_audits(&pool).await,
        [SessionAudit {
            action: "start".to_owned(),
            target_id: id.to_owned(),
            operator_name: "operator".to_owned(),
            client: CLIENT_ADDR.to_owned(),
            change: json!({ "deviceId": PVE_01, "deviceName": "pve-01", "protocol": "ssh" }),
        }]
    );

    let redeemed = redeem(&live, &ticket, now()).expect("the ticket opens the tunnel");
    let pending = redeemed.ticket;
    assert_eq!(pending.session.to_string(), id);
    assert_eq!(pending.device.to_string(), PVE_01);
    assert_eq!(pending.protocol, Protocol::Ssh);
    assert_eq!(pending.operator.name, "operator");
    let display = pending.display;
    assert_eq!(
        (display.width, display.height, display.dpi),
        (1280, 800, 96)
    );
    assert_eq!(pending.timezone.as_deref(), Some("Europe/Berlin"));
    assert_eq!(pending.expires_at, expires_at);
}

/// The only error of `startSession` for `device` with `display`.
async fn refusal(pool: &PgPool, device: &str, display: Value) -> Value {
    let schema = schema_at(pool, now(), &Live::default());
    let response = schema.execute(start_request(device, &display)).await;
    error_json(&response)
}

/// Asserts the refusal left no row and no audit entry.
async fn nothing_written(pool: &PgPool) {
    assert_eq!(session_count(pool).await, 0);
    assert_eq!(session_audits(pool).await.len(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k4_1_start_session_refuses_a_missing_device_with_not_found(pool: PgPool) {
    add_operator(&pool).await;
    let error = refusal(&pool, NO_SUCH_DEVICE, display()).await;
    assert_eq!(error["extensions"]["code"], "NOT_FOUND", "{error}");
    let error = refusal(&pool, "pve-01", display()).await;
    assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
    assert_eq!(error["extensions"]["fields"][0]["field"], "deviceId");
    nothing_written(&pool).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k4_1_start_session_refuses_a_device_with_ssh_turned_off(pool: PgPool) {
    ready(&pool, &[HOME_ASSISTANT]).await;
    let error = refusal(&pool, HOME_ASSISTANT, display()).await;
    assert_eq!(error["extensions"]["code"], "CONFLICT", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("SSH is turned off")
    );
    nothing_written(&pool).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_start_session_refuses_a_device_without_an_accepted_host_key(pool: PgPool) {
    add_operator(&pool).await;
    let error = refusal(&pool, PVE_01, display()).await;
    assert_eq!(error["extensions"]["code"], "CONFLICT", "{error}");
    assert!(error["message"].as_str().unwrap().contains("host key"));
    nothing_written(&pool).await;
}

fn fields(error: &Value) -> Vec<&str> {
    error["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["field"].as_str().unwrap())
        .collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn gw_012_start_session_refuses_a_display_out_of_bounds(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    let all = json!({ "widthPx": 0, "heightPx": 8193, "dpi": 47 });
    let error = refusal(&pool, PVE_01, all).await;
    assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
    assert_eq!(
        fields(&error),
        ["display.widthPx", "display.heightPx", "display.dpi"]
    );
    for (display, field) in [
        (json!({ "widthPx": -1, "heightPx": 800 }), "display.widthPx"),
        (
            json!({ "widthPx": 1280, "heightPx": 0 }),
            "display.heightPx",
        ),
        (
            json!({ "widthPx": 8193, "heightPx": 800 }),
            "display.widthPx",
        ),
        (
            json!({ "widthPx": 1280, "heightPx": 800, "dpi": 481 }),
            "display.dpi",
        ),
    ] {
        let error = refusal(&pool, PVE_01, display).await;
        assert_eq!(fields(&error), [field], "{error}");
    }
    nothing_written(&pool).await;

    // The bounds themselves are accepted.
    let schema = schema_at(&pool, now(), &Live::default());
    for display in [
        json!({ "widthPx": 1, "heightPx": 1, "dpi": 48 }),
        json!({ "widthPx": 8192, "heightPx": 8192, "dpi": 480 }),
        json!({ "widthPx": 1280, "heightPx": 800, "dpi": null }),
    ] {
        let response = schema.execute(start_request(PVE_01, &display)).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_032_e_6_a_time_zone_that_is_not_a_short_zone_name_starts_without_one(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    let at_cap = format!("America/{}", "A".repeat(56));
    // 65 bytes of zone-name characters: only the length is wrong.
    let over_cap = format!("{at_cap}A");
    for (zone, held) in [
        (json!(over_cap), None),
        (json!("Europe/../Berlin"), None),
        (json!(""), None),
        (json!(at_cap), Some(at_cap.as_str())),
        (json!(null), None),
    ] {
        let display = json!({ "widthPx": 1280, "heightPx": 800, "timezone": zone });
        let response = schema.execute(start_request(PVE_01, &display)).await;
        assert!(response.errors.is_empty(), "{zone}: {:?}", response.errors);
        let ticket = response.data.into_json().unwrap()["startSession"].clone();
        let redeemed = redeem(&live, &ticket, now()).unwrap();
        assert_eq!(redeemed.ticket.timezone.as_deref(), held, "{zone}");
    }
}

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
    fixtures(path = "../../fixtures", scripts("devices"))
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
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_030_start_session_refuses_a_host_literal_in_the_deny_list(pool: PgPool) {
    ready(&pool, &[DOCKER_HOST]).await;
    let live = Live::default().denying(AddressPolicy::new(vec!["10.10.0.0/24".parse().unwrap()]));
    let schema = schema_at(&pool, now(), &live);
    for host in ["10.10.0.21", "::ffff:10.10.0.21"] {
        set_host(&pool, DOCKER_HOST, host).await;
        let error = error_json(&schema.execute(start_request(DOCKER_HOST, &display())).await);
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{host}: {error}");
        assert_eq!(fields(&error), ["host"], "{host}");
    }
    nothing_written(&pool).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k4_1_start_session_reports_display_and_host_problems_together(pool: PgPool) {
    ready(&pool, &[DOCKER_HOST]).await;
    set_host(&pool, DOCKER_HOST, "127.0.0.1").await;
    let error = refusal(&pool, DOCKER_HOST, json!({ "widthPx": 0, "heightPx": 800 })).await;
    assert_eq!(fields(&error), ["display.widthPx", "host"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn gw_012_a_null_dpi_means_96(pool: PgPool) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    ready(&pool, &[PVE_01]).await;
    let display = json!({ "widthPx": 1280, "heightPx": 800, "dpi": null });
    let response = schema.execute(start_request(PVE_01, &display)).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let ticket = response.data.into_json().unwrap()["startSession"].clone();
    let redeemed = redeem(&live, &ticket, now()).unwrap();
    assert_eq!(redeemed.ticket.display.dpi, 96);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_009_start_session_is_unavailable_when_the_table_is_full_or_the_hub_stops(
    pool: PgPool,
) {
    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    ready(&pool, &[PVE_01]).await;
    for _ in 0..128 {
        started(&schema, PVE_01).await;
    }
    let full = error_json(&schema.execute(start_request(PVE_01, &display())).await);
    assert_eq!(full["extensions"]["code"], "UNAVAILABLE", "{full}");
    assert_eq!(
        session_count(&pool).await,
        128,
        "the refused start wrote no row"
    );

    let stopping = Live::default();
    sessions::shut_down(&pool, &stopping, now()).await.unwrap();
    let stopped = schema_at(&pool, now(), &stopping);
    let error = error_json(&stopped.execute(start_request(PVE_01, &display())).await);
    assert_eq!(error["extensions"]["code"], "UNAVAILABLE", "{error}");
    assert_eq!(session_count(&pool).await, 128);
}
