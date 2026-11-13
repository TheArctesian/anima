//! `device list` and the device name lookup against a hub that pages badly or lies (CMD-022,
//! CMD-026).

use serde_json::json;

use super::devices::{fixture_devices, page};
use crate::kit::Anima;
use crate::kit::fake_hub::FakeHub;

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-device-paging-tests")
}

#[tokio::test]
async fn cmd_026_s4_a_name_lookup_stops_on_a_hub_that_never_ends() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer("DeviceList", page(&devices[..1], Some("c-loop"), 9));
    let run = signed_in()
        .run(&["device", "get", "no-such-name", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("Stopped paging"), "{}", run.stderr);
}

#[tokio::test]
async fn cmd_026_c3_the_next_page_hint_keeps_the_filters() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    hub.answer("DeviceList", page(&devices[..1], Some("c-1"), 9));
    let run = signed_in()
        .run(&[
            "device",
            "list",
            "--status",
            "down",
            "--tag",
            "infra",
            "--name",
            "my pve",
            "--sort",
            "status",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert!(
        run.stderr.contains(
            "anima device list --status down --tag infra --name 'my pve' --sort status --after c-1"
        ),
        "{}",
        run.stderr
    );
}

#[tokio::test]
async fn cmd_022_s2_an_id_from_the_hub_never_reaches_the_terminal_raw() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    let mut twin = devices[0].clone();
    twin["id"] = json!("evil\u{1b}]0;pwned\u{7}\u{202E}id");
    hub.answer(
        "DeviceList",
        page(&[twin.clone(), devices[0].clone()], None, 2),
    );
    let name = devices[0]["name"].as_str().expect("name");
    let run = signed_in()
        .run(&["device", "get", name, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(run.stderr.contains("hint:"), "{}", run.stderr);
    assert!(
        !run.stderr.contains(['\u{1b}', '\u{7}', '\u{202E}']),
        "{:?}",
        run.stderr
    );
}
