//! The location tools of the `records` set against the scripted hub (MCP-002, MCP-017; K-E4
//! Sections 11 to 13, R-145). No tool makes or returns an ingest token (CMD-030).

use serde_json::{Value, json};

use crate::kit::fake_hub::data;
use crate::mcp::engine::{sent, structured, writing};
use crate::mcp::session::{Desk, LOGIN_TOKEN, hub};

const EVENT: &str = "0190a000-0000-7000-9300-000000000001";

fn sample() -> Value {
    json!({ "id": "0190a000-0000-7000-9300-000000000002", "provider": "OWNTRACKS",
            "account": "phone", "key": "ph:1794830400",
            "observedAt": "2026-11-16T12:00:00+00:00",
            "receivedAt": "2026-11-16T12:00:05+00:00", "latitude": 52.0, "longitude": 4.0,
            "accuracyM": null, "altitudeM": null, "speedMps": null })
}

fn context() -> Value {
    let unknown = json!({ "deviceId": null, "siteId": null, "placeId": null });
    json!({ "eventId": EVENT, "occurredAt": "2026-11-16T12:00:00+00:00",
            "direction": "DOWNLOAD", "actorKind": "HUMAN", "actor": "operator",
            "clientDeviceId": null,
            "source": { "deviceId": "d", "siteId": "s", "placeId": "office" },
            "destination": unknown,
            "operator": { "basis": "NEAREST_INFERRED", "sampleId": "x", "offsetSeconds": 300,
                          "uncertain": true, "placeId": null } })
}

#[tokio::test]
async fn cmd_030_no_tool_makes_or_returns_an_ingest_token() {
    let hub = hub().await;
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let names = session.tool_names().await;
    for name in &names {
        assert!(!name.contains("token"), "{name}");
    }
    assert!(names.iter().any(|name| name == "anima_location_import"));
    session.finish().await;
}

#[tokio::test]
async fn mcp_002_the_import_tool_is_off_until_writing_is_allowed() {
    let hub = hub().await;
    let mut reading = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let names = reading.tool_names().await;
    assert!(names.iter().any(|name| name == "anima_locations_list"));
    assert!(names.iter().any(|name| name == "anima_location_context"));
    assert!(!names.iter().any(|name| name == "anima_location_import"));
    reading.finish().await;
}

#[tokio::test]
async fn mcp_017_the_location_tools_list_import_and_give_the_context() {
    let hub = hub().await;
    hub.answer(
        "LocationList",
        data(json!({ "locationSamples": [sample()] })),
    );
    hub.answer(
        "LocationRecord",
        data(json!({ "recordLocationSamples": 1 })),
    );
    hub.answer(
        "LocationContext",
        data(json!({ "transferContext": context() })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;

    let span = json!({ "from": "2026-11-16T00:00:00Z", "to": "2026-11-17T00:00:00Z" });
    let listed = session.call("anima_locations_list", span).await;
    let shown = &structured(&listed)["samples"][0];
    assert_eq!(shown["key"], "ph:1794830400");
    assert!(shown["accuracyM"].is_null(), "unknown stays null: {shown}");

    let samples = json!({ "samples": [{ "key": "a", "observedAt": "2026-11-16T12:00:00Z",
                                         "latitude": 52.0, "longitude": 4.0 }] });
    let imported = session.call("anima_location_import", samples).await;
    assert_eq!(structured(&imported)["added"], 1);
    let sent_sample = &sent(&hub, "LocationRecord")[0]["samples"][0];
    assert_eq!(sent_sample["key"], "a");
    assert!(sent_sample["accuracyM"].is_null(), "no number in its place");

    let read = session
        .call("anima_location_context", json!({ "event": EVENT }))
        .await;
    let context = &structured(&read)["context"];
    assert_eq!(context["source"]["placeId"], "office");
    assert_eq!(context["operator"]["basis"], "NEAREST_INFERRED");
    assert!(context["operator"]["placeId"].is_null());
    session.finish().await;
}
