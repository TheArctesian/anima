//! A hosted hub keeps the check `Connected` for a computer on its link (hosted-mode.md
//! HMD-047, R-143): the serving hub asks the control socket of the link, adds the check to
//! the device whose host is the link address of a connected computer and reports it as Up.
//! A computer that the link never saw gets no check.

use std::time::{Duration, Instant};

use anima_hub::{
    fleet::checks::{self, OwnCheck},
    ids::DeviceId,
    status::Status,
};
use chrono::{SecondsFormat, Utc};
use serde_json::json;
use sqlx::PgPool;

use super::{
    control::{FakeLink, KEY, ok},
    hub_device::{DEADLINE, Home},
};

/// `docker-host` and `ollama` of the K6 fixtures.
const LAPTOP: &str = "0190a000-0000-7000-8002-000000000002";
const DESKTOP: &str = "0190a000-0000-7000-8002-000000000003";

/// The `devices.list` reply of a link with a laptop that is connected now and a desktop
/// that never connected.
fn two_computers() -> String {
    let device = |name: &str, address: &str| {
        json!({
            "key": KEY, "fingerprint": "1a2b3c4d", "name": name, "address": address,
            "permitted": "2030-05-05T12:00:00Z", "expires": "2030-08-03T12:00:00Z",
            "streams": 0, "path": "direct",
        })
    };
    let mut laptop = device("laptop", "fd41:4e49:4d41::7");
    laptop["seen"] = json!(Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true));
    laptop["streams"] = json!(1);
    ok(&json!([laptop, device("desktop", "fd41:4e49:4d41::8")]))
}

async fn set_host(pool: &PgPool, id: &str, host: &str) {
    sqlx::query("update devices.devices set host = $2 where id = $1::uuid")
        .bind(id)
        .bind(host)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn hmd_047_a_hosted_hub_reports_a_connected_computer_of_its_link_as_up(pool: PgPool) {
    let (laptop, desktop): (DeviceId, DeviceId) =
        (LAPTOP.parse().unwrap(), DESKTOP.parse().unwrap());
    set_host(&pool, LAPTOP, "fd41:4e49:4d41::7").await;
    set_host(&pool, DESKTOP, "fd41:4e49:4d41::8").await;
    let link = FakeLink::start(|_| two_computers());
    let home = Home::new(&pool).await;

    let mut hub = home.start_hosted_beside(&link.path);
    let started = Instant::now();
    let check = loop {
        let found = checks::own::find(&pool, laptop, OwnCheck::Connected).await;
        let status = checks::device_status(&pool, laptop, Utc::now()).await;
        if let (Some(check), Status::Up) = (found.unwrap(), status.unwrap()) {
            break check;
        }
        assert!(hub.runs(), "the hub stopped");
        assert!(started.elapsed() < DEADLINE, "not before the deadline");
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    let stored = checks::get(&pool, check).await.unwrap();
    assert_eq!(stored.name, "Connected");
    assert!(stored.push_token.is_some(), "a PUSH check");
    let of_laptop = checks::for_device(&pool, laptop).await.unwrap();
    assert_eq!(of_laptop.len(), 1);
    // The desktop never connected: no check, so its row stays as the application made it.
    let of_desktop = checks::for_device(&pool, desktop).await.unwrap();
    assert_eq!(of_desktop.len(), 0);
    // The hub only reads the link.
    assert_eq!(link.changes(), Vec::<serde_json::Value>::new());
    assert_eq!(link.requests()[0], json!({ "verb": "devices.list" }));
}
