//! The batteries of a device in `anima device get` and in `anima_device_get` (MET-032,
//! CMD-030): the last state from the device, the charge from its `BATTERY_PERCENT` series.

use serde_json::{Value, json};

use super::{detail, fixture_devices, signed_in};
use crate::kit::fake_hub::{FakeHub, data};
use crate::mcp::session::{Desk, LOGIN_TOKEN, hub};

/// A laptop: one battery that discharges, one that is full and has no reading this hour.
fn laptop() -> (Value, Value) {
    let devices = fixture_devices();
    let mut device = detail(&devices[0]);
    device["data"]["device"]["batteries"] = json!([
        { "name": "BAT0", "state": "DISCHARGING", "secondsLeft": 7500, "healthPercent": 92 },
        { "name": "BAT1", "state": "FULL", "secondsLeft": null, "healthPercent": null },
    ]);
    let points = |values: &[f64]| -> Vec<Value> {
        values
            .iter()
            .map(|value| json!({ "value": value }))
            .collect()
    };
    let charge = data(json!({ "device": { "metrics": [
        { "kind": "CPU_PERCENT", "label": "BAT0", "points": points(&[3.0]) },
        { "kind": "BATTERY_PERCENT", "label": "BAT0", "points": points(&[90.0, 87.0]) },
    ] } }));
    (device, charge)
}

#[tokio::test]
async fn met_032_device_get_prints_each_battery_with_its_charge_and_state() {
    let hub = FakeHub::start().await;
    let (device, charge) = laptop();
    let id = device["data"]["device"]["id"]
        .as_str()
        .expect("an id")
        .to_owned();
    hub.answer("DeviceGet", device)
        .answer("DeviceBatteryCharge", charge);
    let run = signed_in()
        .run(&["device", "get", &id, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let line = |name: &str| {
        run.stdout
            .lines()
            .find(|line| line.starts_with(&format!("Battery {name}")))
            .map(|line| {
                line.split_whitespace()
                    .skip(2)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
    };
    assert_eq!(
        (line("BAT0"), line("BAT1")),
        (
            Some("87 %, discharging, 2 h 5 min left, health 92 %".to_owned()),
            Some("-, full".to_owned())
        ),
        "{}",
        run.stdout
    );

    let run = signed_in()
        .run(&["device", "get", &id, "--json", "--hub", &hub.origin])
        .await;
    let batteries = &run.json()["batteries"];
    assert_eq!(
        (
            batteries[0]["chargePercent"].clone(),
            batteries[0]["state"].clone()
        ),
        (json!(87.0), json!("DISCHARGING"))
    );
    assert_eq!(batteries[1]["chargePercent"], Value::Null);
}

#[tokio::test]
async fn met_032_a_device_with_no_battery_prints_none_and_asks_for_no_charge() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer("DeviceGet", detail(&devices[0]));
    let id = devices[0]["id"].as_str().expect("an id");
    let run = signed_in()
        .run(&["device", "get", id, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(!run.stdout.contains("Battery"), "{}", run.stdout);
    let run = signed_in()
        .run(&["device", "get", id, "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()["batteries"], json!([]));
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "DeviceBatteryCharge")
    );
}

#[tokio::test]
async fn met_032_the_device_tool_gives_the_batteries_with_their_charge() {
    let hub = hub().await;
    let (device, charge) = laptop();
    let id = device["data"]["device"]["id"]
        .as_str()
        .expect("an id")
        .to_owned();
    let web = data(
        json!({ "device": { "id": id, "name": device["data"]["device"]["name"],
        "webInterfaces": [], "webInterfaceSuggestions": [] } }),
    );
    hub.answer("DeviceGet", device)
        .answer("DeviceBatteryCharge", charge)
        .answer("WebList", web);
    let desk = Desk::new();
    let (mut session, _) = desk.start(&hub, LOGIN_TOKEN, &[]).await;
    let answer = session
        .call("anima_device_get", json!({ "device": id }))
        .await;
    assert_eq!(answer["isError"], false, "{answer}");
    let batteries = &answer["structuredContent"]["device"]["batteries"];
    assert_eq!(
        (
            batteries[0]["name"].clone(),
            batteries[0]["chargePercent"].clone(),
            batteries[0]["state"].clone(),
            batteries[1]["state"].clone(),
        ),
        (
            json!("BAT0"),
            json!(87.0),
            json!("DISCHARGING"),
            json!("FULL")
        ),
        "{answer}"
    );
    session.finish().await;
}
