//! `anima maintenance` against the scripted hub (CMD-020, CMD-024).

use serde_json::{Value, json};

use crate::fleet::devices::{fixture_devices, page};
use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const WINDOW: &str = "0190a000-0000-7000-8004-000000000001";
const MANUAL: &str = "0190a000-0000-7000-8004-000000000009";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-maintenance-tests")
}

fn window(id: &str, name: &str, kind: &str, extra: &Value) -> Value {
    let mut window = json!({
        "id": id, "name": name, "kind": kind, "timezone": "Pacific/Auckland",
        "activeManual": null, "startsAt": null, "endsAt": null, "firstStartAt": null,
        "everyDays": null, "weekdays": null, "daysOfMonth": null, "startTime": null,
        "durationMinutes": null, "targets": [{ "type": "TAG", "target": "infra" }],
        "activeNow": false,
    });
    for (key, value) in extra.as_object().expect("an object") {
        window[key] = value.clone();
    }
    window
}

fn listed() -> Value {
    data(json!({ "maintenanceWindows": [
        window(WINDOW, "Sunday updates", "RECURRING_WEEKDAY",
            &json!({ "weekdays": ["SUN"], "startTime": "03:00", "durationMinutes": 60 })),
        window(MANUAL, "Hands on", "MANUAL", &json!({ "activeManual": false })),
    ]}))
}

fn changed(field: &str, active: bool) -> Value {
    data(
        json!({ field: { "id": MANUAL, "name": "Hands on", "kind": "MANUAL", "activeNow": active } }),
    )
}

fn sent(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .map_or(json!(null), |seen| seen.variables)
}

#[tokio::test]
async fn cmd_020_maintenance_list_prints_each_schedule_in_words() {
    let hub = FakeHub::start().await;
    hub.answer("MaintenanceList", listed());
    let run = signed_in()
        .run(&["maintenance", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let sunday = run
        .stdout
        .lines()
        .find(|l| l.starts_with("Sunday"))
        .expect("a row");
    for expected in [
        "RECURRING_WEEKDAY",
        "SUN at 03:00 for 60 min",
        "tag:infra",
        WINDOW,
    ] {
        assert!(sunday.contains(expected), "{expected}: {sunday}");
    }
    let run = signed_in()
        .run(&["maintenance", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()[1]["kind"], "MANUAL");
}

#[tokio::test]
async fn maintenance_create_maps_each_flag_to_its_k1_input_field() {
    let hub = FakeHub::start().await;
    hub.answer(
        "MaintenanceCreate",
        changed("createMaintenanceWindow", false),
    );
    let run = signed_in()
        .run(&[
            "maintenance",
            "create",
            "--name",
            "Nightly",
            "--kind",
            "recurring-weekday",
            "--timezone",
            "Europe/Berlin",
            "--weekdays",
            "mon,fri",
            "--start-time",
            "01:00",
            "--duration-minutes",
            "30",
            "--targets",
            "device:0190a000-0000-7000-8002-000000000001,tag:infra",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        sent(&hub, "MaintenanceCreate")["input"],
        json!({
            "name": "Nightly", "kind": "RECURRING_WEEKDAY", "timezone": "Europe/Berlin",
            "weekdays": ["MON", "FRI"], "startTime": "01:00", "durationMinutes": 30,
            "targets": [{ "type": "DEVICE", "target": "0190a000-0000-7000-8002-000000000001" }, { "type": "TAG", "target": "infra" }],
        })
    );

    let run = signed_in()
        .run(&["maintenance", "create", "--name", "x", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--kind"), "{}", run.stderr);
}

#[tokio::test]
async fn maintenance_update_keeps_the_fields_it_is_not_given() {
    let hub = FakeHub::start().await;
    hub.answer("MaintenanceList", listed()).answer(
        "MaintenanceUpdate",
        changed("updateMaintenanceWindow", false),
    );
    let run = signed_in()
        .run(&[
            "maintenance",
            "update",
            "Sunday updates",
            "--duration-minutes",
            "90",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let update = sent(&hub, "MaintenanceUpdate");
    assert_eq!(update["id"], WINDOW);
    assert_eq!(
        update["input"],
        json!({
            "name": "Sunday updates", "kind": "RECURRING_WEEKDAY", "timezone": "Pacific/Auckland",
            "weekdays": ["SUN"], "startTime": "03:00", "durationMinutes": 90,
            "targets": [{ "type": "TAG", "target": "infra" }],
        })
    );
}

#[tokio::test]
async fn cmd_024_maintenance_delete_needs_a_confirmation() {
    let hub = FakeHub::start().await;
    hub.answer("MaintenanceList", listed()).answer(
        "MaintenanceDelete",
        data(json!({ "deleteMaintenanceWindow": WINDOW })),
    );
    let run = signed_in()
        .run(&[
            "maintenance",
            "delete",
            WINDOW,
            "--no-input",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("--yes"), "{}", run.stderr);
    assert!(
        hub.seen()
            .iter()
            .all(|seen| seen.operation != "MaintenanceDelete")
    );

    let run = signed_in()
        .run(&[
            "maintenance",
            "delete",
            WINDOW,
            "--yes",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(sent(&hub, "MaintenanceDelete")["id"], WINDOW);
}

#[tokio::test]
async fn maintenance_start_and_stop_turn_a_manual_window_on_and_off() {
    let hub = FakeHub::start().await;
    hub.answer("MaintenanceList", listed())
        .answer(
            "MaintenanceSetActive",
            changed("setManualWindowActive", true),
        )
        .answer(
            "MaintenanceSetActive",
            changed("setManualWindowActive", false),
        );
    let run = signed_in()
        .run(&["maintenance", "start", "Hands on", "--hub", &hub.origin])
        .await;
    assert!(run.stdout.contains("yes"), "{run:?}");
    let run = signed_in()
        .run(&["maintenance", "stop", "Hands on", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let active: Vec<Value> = hub
        .seen()
        .iter()
        .filter(|seen| seen.operation == "MaintenanceSetActive")
        .map(|seen| seen.variables["active"].clone())
        .collect();
    assert_eq!(active, [json!(true), json!(false)]);
}

#[tokio::test]
async fn cmd_021_maintenance_refusals_and_unknown_windows_have_their_exit_codes() {
    let hub = FakeHub::start().await;
    hub.answer("MaintenanceList", listed()).answer(
        "MaintenanceSetActive",
        refusal("FORBIDDEN", "Read-only token."),
    );
    let run = signed_in()
        .run(&["maintenance", "start", MANUAL, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 5, "{run:?}");
    let run = signed_in()
        .run(&[
            "maintenance",
            "stop",
            "No such window",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
}

#[tokio::test]
async fn cmd_021_c9_maintenance_update_with_no_flag_is_exit_2_and_sends_nothing() {
    let hub = FakeHub::start().await;
    hub.answer("MaintenanceList", listed());
    let run = signed_in()
        .run(&["maintenance", "update", WINDOW, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("Nothing to change"), "{}", run.stderr);
    assert!(hub.seen().is_empty(), "no request of any kind");
}

#[tokio::test]
async fn cmd_024_c10_a_device_target_may_be_an_exact_name() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer("DeviceList", page(&devices[..1], None, 1))
        .answer(
            "MaintenanceCreate",
            changed("createMaintenanceWindow", false),
        );
    let name = devices[0]["name"].as_str().expect("name");
    let target = format!("device:{name}");
    let run = signed_in()
        .run(&[
            "maintenance",
            "create",
            "--name",
            "Nightly",
            "--kind",
            "manual",
            "--timezone",
            "Europe/Berlin",
            "--targets",
            &target,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        sent(&hub, "MaintenanceCreate")["input"]["targets"],
        json!([{ "type": "DEVICE", "target": devices[0]["id"] }])
    );
}
