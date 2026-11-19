//! `anima location` against the scripted hub (K-E4 Sections 11 to 13; R-145, CMD-014).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

const EVENT: &str = "0190a000-0000-7000-9300-000000000001";
const TOKEN: &str = "anl_location-ingest-token-of-the-test";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-location-tests")
}

fn sample(accuracy: &Value) -> Value {
    json!({ "id": "0190a000-0000-7000-9300-000000000002", "provider": "DAWARICH",
            "account": "home", "key": "41", "observedAt": "2026-11-16T12:00:00+00:00",
            "receivedAt": "2026-11-17T08:00:00+00:00", "latitude": 52.123_45,
            "longitude": 4.5, "accuracyM": accuracy, "altitudeM": null, "speedMps": null })
}

#[tokio::test]
async fn r_145_location_list_sends_the_span_and_shows_a_missing_accuracy_as_unknown() {
    let hub = FakeHub::start().await;
    let samples = json!([sample(&json!(12.0)), sample(&Value::Null)]);
    hub.answer("LocationList", data(json!({ "locationSamples": samples })));
    let run = signed_in()
        .run(&[
            "location",
            "list",
            "--from",
            "2026-11-16T00:00:00Z",
            "--to",
            "2026-11-17T00:00:00+00:00",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(
        hub.seen()[0].variables,
        json!({ "from": "2026-11-16T00:00:00.000000Z", "to": "2026-11-17T00:00:00.000000Z",
                "first": 1000 })
    );
    let rows: Vec<&str> = run
        .stdout
        .lines()
        .filter(|l| l.contains("DAWARICH"))
        .collect();
    assert_eq!(rows.len(), 2, "{}", run.stdout);
    assert!(
        rows[0].contains("52.12345") && rows[0].contains(" 12 "),
        "{}",
        rows[0]
    );
    assert!(rows[1].contains(" ? "), "{}", rows[1]);
}

#[tokio::test]
async fn r_145_location_import_sends_the_samples_of_a_file_and_says_how_many_were_new() {
    let hub = FakeHub::start().await;
    hub.answer(
        "LocationRecord",
        data(json!({ "recordLocationSamples": 1 })),
    );
    let folder = tempfile::tempdir().expect("a folder");
    let file = folder.path().join("fixes.jsonl");
    let lines = "{\"_type\":\"location\",\"tst\":1794830400,\"lat\":52.1,\"lon\":4.2,\"tid\":\"ph\"}\n\
                 {\"_type\":\"lwt\",\"tst\":1}\n{\"key\":\"b\"}\n";
    std::fs::write(&file, lines).expect("the file");
    let run = signed_in()
        .run(&[
            "location",
            "import",
            file.to_str().expect("a path"),
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains("2 sample(s) read, 1 new"),
        "{}",
        run.stdout
    );
    let sent = &hub.seen()[0].variables["samples"];
    assert_eq!(sent.as_array().map(Vec::len), Some(2), "{sent}");
    assert_eq!(sent[0]["key"], "ph:1794830400");
    assert_eq!(sent[0]["observedAt"], "2026-11-16T12:00:00Z");
    assert!(sent[0]["accuracyM"].is_null());
}

#[tokio::test]
async fn r_145_location_import_of_an_unusable_file_asks_the_hub_nothing() {
    let hub = FakeHub::start().await;
    let folder = tempfile::tempdir().expect("a folder");
    let file = folder.path().join("fixes.jsonl");
    std::fs::write(&file, "{\"key\":\"a\"}\nsecret text that is not json\n").expect("the file");
    let run = signed_in()
        .run(&[
            "location",
            "import",
            file.to_str().expect("a path"),
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_ne!(run.code, 0);
    assert!(run.stderr.contains("entry 2 is not JSON"), "{}", run.stderr);
    assert!(!run.stderr.contains("secret text"), "{}", run.stderr);
    assert!(hub.seen().is_empty());
}

#[tokio::test]
async fn cmd_014_location_token_add_shows_the_token_one_time_on_standard_output() {
    let hub = FakeHub::start().await;
    let issued = json!({ "token": TOKEN, "ingestToken": {
        "id": "0190a000-0000-7000-9300-000000000003", "name": "Phone",
        "createdAt": "2026-11-18T10:00:00+00:00" } });
    hub.answer(
        "LocationTokenAdd",
        data(json!({ "createLocationIngestToken": issued })),
    );
    let run = signed_in()
        .run(&["location", "token", "add", "Phone", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.stdout.trim(), TOKEN);
    assert!(run.stderr.contains("shown one time"), "{}", run.stderr);
    assert!(
        run.stderr.contains("works until you revoke it"),
        "{}",
        run.stderr
    );
    assert!(!run.stderr.contains(TOKEN), "{}", run.stderr);
    assert_eq!(hub.seen()[0].variables, json!({ "name": "Phone" }));
}

#[tokio::test]
async fn r_145_location_context_keeps_the_device_and_the_operator_apart() {
    let hub = FakeHub::start().await;
    let unknown = json!({ "deviceId": null, "siteId": null, "placeId": null });
    let context = json!({ "eventId": EVENT, "occurredAt": "2026-11-16T12:00:00+00:00",
        "direction": "DOWNLOAD", "actorKind": "HUMAN", "actor": "operator",
        "clientDeviceId": null,
        "source": { "deviceId": "device-1", "siteId": "site-1", "placeId": "office-1" },
        "destination": unknown,
        "operator": { "basis": "NEAREST_INFERRED", "sampleId": "sample-1",
                      "offsetSeconds": 300, "uncertain": false, "placeId": null } });
    hub.answer(
        "LocationContext",
        data(json!({ "transferContext": context })),
    );
    let run = signed_in()
        .run(&["location", "context", EVENT, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let line = |start: &str| {
        let found = run.stdout.lines().find(|l| l.starts_with(start));
        found
            .unwrap_or_else(|| panic!("no line {start}: {}", run.stdout))
            .to_owned()
    };
    assert!(line("source place").contains("office-1"));
    assert!(line("destination device").contains("unknown"));
    assert!(line("operator evidence").contains("inferred from the nearest sample (+300 s)"));
    assert!(line("operator place").contains("unknown"));
}

#[tokio::test]
async fn r_145_location_context_of_an_event_that_is_no_transfer_is_not_found() {
    let hub = FakeHub::start().await;
    hub.answer("LocationContext", data(json!({ "transferContext": null })));
    let run = signed_in()
        .run(&["location", "context", EVENT, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 3, "{run:?}");
    assert!(run.stderr.contains("not a file transfer"), "{}", run.stderr);
}
