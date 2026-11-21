//! `anima device get` against the scripted hub: by ID or exact name, and the exit codes of a
//! device that is not there or not one (CMD-020, CMD-021).

use serde_json::json;

use super::{detail, fixture_devices, page, signed_in};
use crate::kit::fake_hub::{FakeHub, data, refusal};

#[tokio::test]
async fn device_get_finds_a_device_by_its_exact_name() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    let mut longer = devices[0].clone();
    longer["name"] = json!(format!(
        "{}-backup",
        devices[0]["name"].as_str().expect("name")
    ));
    longer["id"] = json!("0190a000-0000-7000-8002-0000000000ff");
    hub.answer("DeviceList", page(&[devices[0].clone(), longer], None, 2))
        .answer("DeviceGet", detail(&devices[0]));
    let name = devices[0]["name"].as_str().expect("name");
    let run = signed_in()
        .run(&["device", "get", name, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let name_line = run.stdout.lines().find(|line| line.starts_with("Name"));
    assert!(
        name_line.is_some_and(|line| line.ends_with(name)),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("SHA256:abc"), "{}", run.stdout);
    let get = hub
        .seen()
        .into_iter()
        .find(|seen| seen.operation == "DeviceGet")
        .expect("a get");
    assert_eq!(get.variables["id"], devices[0]["id"]);

    let run = signed_in()
        .run(&["device", "get", name, "--json", "--hub", &hub.origin])
        .await;
    let json = run.json();
    assert_eq!(
        (json["id"].clone(), json["ssh"]["credentialStored"].clone()),
        (devices[0]["id"].clone(), json!(true))
    );
}

#[tokio::test]
async fn device_get_with_an_ambiguous_name_lists_the_matches() {
    let hub = FakeHub::start().await;
    let devices = fixture_devices();
    let mut twin = devices[0].clone();
    twin["id"] = json!("0190a000-0000-7000-8002-0000000000fe");
    hub.answer(
        "DeviceList",
        page(&[devices[0].clone(), twin.clone()], None, 2),
    );
    let name = devices[0]["name"].as_str().expect("name");
    let run = signed_in()
        .run(&["device", "get", name, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 2, "{run:?}");
    assert!(
        run.stderr.contains(devices[0]["id"].as_str().expect("id")),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains(twin["id"].as_str().expect("id")),
        "{}",
        run.stderr
    );
    assert!(hub.seen().iter().all(|seen| seen.operation != "DeviceGet"));
}

#[tokio::test]
async fn cmd_021_a_device_that_is_not_there_is_exit_3() {
    let hub = FakeHub::start().await;
    hub.answer("DeviceGet", data(json!({ "device": null })))
        .answer("DeviceList", page(&[], None, 0));
    let id = "0190a000-0000-7000-8002-00000000dead";
    let run = signed_in()
        .run(&["device", "get", id, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
    assert!(run.stderr.contains(id));

    let run = signed_in()
        .run(&[
            "device",
            "get",
            "no-such-name",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(run.error()["code"], "NOT_FOUND");

    let refusing = FakeHub::start().await;
    refusing.answer("DeviceGet", refusal("NOT_FOUND", "No such device."));
    let run = signed_in()
        .run(&["device", "get", id, "--hub", &refusing.origin])
        .await;
    assert_eq!(
        (run.code, run.stderr.contains("No such device.")),
        (3, true),
        "{run:?}"
    );
}
