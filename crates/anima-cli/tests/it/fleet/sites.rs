//! `anima site` and `anima tag` against the scripted hub (CMD-020, CMD-021).

use serde_json::{Value, json};

use crate::fleet::devices::{fixture_devices, page};
use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const TOKEN: &str = "login-token-for-site-tests";
const SITE: &str = "0190a000-0000-7000-8001-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", TOKEN)
}

fn site(relay: Option<&str>) -> Value {
    json!({
        "id": SITE, "name": "home", "subnet": "10.10.0.0/24",
        "wakeRelay": relay.map(|name| json!({ "id": "0190a000-0000-7000-8002-000000000001", "name": name })),
    })
}

fn variables(hub: &FakeHub, operation: &str) -> Value {
    hub.seen()
        .into_iter()
        .find(|seen| seen.operation == operation)
        .unwrap_or_else(|| panic!("no {operation} request"))
        .variables
}

#[tokio::test]
async fn cmd_020_site_list_prints_a_row_for_each_site() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SiteList",
        data(json!({ "sites": [site(Some("pi-home")), site(None)] })),
    );
    let run = signed_in()
        .run(&["site", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let header = run.stdout.lines().next().expect("a header");
    assert!(
        header.starts_with("NAME") && header.contains("SUBNET") && header.contains("WAKE RELAY")
    );
    assert!(
        run.stdout
            .lines()
            .any(|line| line.contains("10.10.0.0/24") && line.contains("pi-home"))
    );

    let run = signed_in()
        .run(&["site", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json()[0]["wakeRelay"]["name"], "pi-home");
}

#[tokio::test]
async fn cmd_020_site_create_sends_the_name_and_subnet() {
    let hub = FakeHub::start().await;
    hub.answer("SiteCreate", data(json!({ "createSite": site(None) })));
    let run = signed_in()
        .run(&[
            "site",
            "create",
            "home",
            "--subnet",
            "10.10.0.0/24",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        variables(&hub, "SiteCreate"),
        json!({ "name": "home", "subnet": "10.10.0.0/24" })
    );
    assert!(run.stdout.contains(SITE), "{}", run.stdout);
}

#[tokio::test]
async fn cmd_020_set_wake_relay_takes_a_site_and_device_by_name_or_clears_the_relay() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    let name = devices[0]["name"].as_str().expect("name");
    hub.answer(
        "SiteNames",
        data(json!({ "sites": [{ "id": SITE, "name": "home" }] })),
    )
    .answer("DeviceList", page(&devices, None, devices.len()))
    .answer(
        "SiteSetWakeRelay",
        data(json!({ "setSiteWakeRelay": site(Some(name)) })),
    );
    let run = signed_in()
        .run(&[
            "site",
            "set-wake-relay",
            "home",
            "--device",
            name,
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        variables(&hub, "SiteSetWakeRelay"),
        json!({ "siteId": SITE, "deviceId": devices[0]["id"] })
    );

    let cleared = FakeHub::start().await;
    cleared.answer(
        "SiteSetWakeRelay",
        data(json!({ "setSiteWakeRelay": site(None) })),
    );
    let run = signed_in()
        .run(&[
            "site",
            "set-wake-relay",
            SITE,
            "--clear",
            "--hub",
            &cleared.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        variables(&cleared, "SiteSetWakeRelay"),
        json!({ "siteId": SITE, "deviceId": null })
    );

    let run = signed_in()
        .run(&["site", "set-wake-relay", SITE, "--hub", &cleared.origin])
        .await;
    assert_eq!(
        run.code, 2,
        "one of --device and --clear is needed: {run:?}"
    );
}

#[tokio::test]
async fn cmd_021_a_site_the_hub_refuses_is_its_exit_code() {
    let hub = FakeHub::start().await;
    hub.answer(
        "SiteCreate",
        refusal("CONFLICT", "A site with this name exists."),
    );
    let run = signed_in()
        .run(&[
            "site",
            "create",
            "home",
            "--subnet",
            "10.10.0.0/24",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 6, "{run:?}");
}

#[tokio::test]
async fn cmd_020_tag_list_prints_the_tags() {
    let hub = FakeHub::start().await;
    hub.answer("TagList", data(json!({ "tags": ["infra", "lab"] })));
    let run = signed_in()
        .run(&["tag", "list", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        ["TAG", "infra", "lab"]
    );

    let run = signed_in()
        .run(&["tag", "list", "--json", "--hub", &hub.origin])
        .await;
    assert_eq!(run.json(), json!(["infra", "lab"]));
}
