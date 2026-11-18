//! The `timeline` and `records` tool sets against the scripted hub (MCP-002, MCP-017;
//! TML-007, RLK-001, RLK-002) and the place tools (PLC-001, PLC-004 to PLC-006).

use serde_json::{Value, json};

use super::engine::{sent, structured, writing};
use super::session::{Desk, LOGIN_TOKEN, hub};
use crate::kit::fake_hub::data;

const PERSON: &str = "person:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d";
const ID: &str = "0190a000-0000-7000-9100-000000000001";

fn link() -> Value {
    json!({ "id": ID, "source": "book:1", "target": PERSON, "kind": "lent_to",
            "label": "Ada Lovelace", "sourceTitle": null, "targetTitle": null,
            "createdAt": "2026-11-16T10:00:00+00:00" })
}

#[tokio::test]
async fn mcp_017_timeline_list_returns_the_entries_and_the_next_cursor() {
    let hub = hub().await;
    let node = json!({ "reference": "event:1", "source": "event_log", "category": "DEVICE",
        "title": "Ping is down.", "start": "2026-11-06T08:00:00+00:00", "end": null,
        "receivedAt": null,
        "basis": "OBSERVED", "uncertainty": ["End time not given by the source."],
        "supersededBy": null });
    hub.answer(
        "TimelineList",
        data(json!({ "timeline": {
            "edges": [{ "cursor": "c-1", "node": node }],
            "pageInfo": { "hasNextPage": true, "endCursor": "c-1" } } })),
    );
    let mut session = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let arguments = json!({ "from": "2026-11-06T00:00:00Z", "to": "2026-11-07T00:00:00Z",
                            "category": ["DEVICE"], "limit": 1 });
    let answer = session.call("anima_timeline_list", arguments).await;
    let result = structured(&answer);
    assert_eq!(result["entries"][0]["reference"], "event:1");
    assert_eq!(
        result["entries"][0]["uncertainty"][0],
        "End time not given by the source."
    );
    assert_eq!(result["nextCursor"], "c-1");
    let variables = &sent(&hub, "TimelineList")[0];
    assert_eq!(variables["categories"], json!(["DEVICE"]));
    assert_eq!(variables["first"], 1);
    session.finish().await;
}

#[tokio::test]
async fn mcp_002_the_link_tools_that_change_are_off_until_writing_is_allowed() {
    let hub = hub().await;
    let mut reading = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let names = reading.tool_names().await;
    assert!(names.iter().any(|name| name == "anima_timeline_list"));
    assert!(names.iter().any(|name| name == "anima_record_links_list"));
    assert!(!names.iter().any(|name| name == "anima_record_link_add"));
    assert!(!names.iter().any(|name| name == "anima_record_link_remove"));
    reading.finish().await;
}

#[tokio::test]
async fn mcp_017_the_link_tools_list_add_and_remove_a_link() {
    let hub = hub().await;
    hub.answer("RecordLinkList", data(json!({ "recordLinks": [link()] })));
    hub.answer("RecordLinkAdd", data(json!({ "linkRecords": link() })));
    hub.answer("RecordLinkRemove", data(json!({ "unlinkRecords": ID })));
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;

    let listed = session
        .call("anima_record_links_list", json!({ "record": "book:1" }))
        .await;
    assert_eq!(structured(&listed)["links"][0]["label"], "Ada Lovelace");

    let arguments = json!({ "source": "book:1", "target": PERSON, "kind": "lent_to",
                            "label": "Ada Lovelace" });
    let added = session
        .call("anima_record_link_add", arguments.clone())
        .await;
    assert_eq!(structured(&added)["link"]["id"], ID);
    assert_eq!(sent(&hub, "RecordLinkAdd")[0]["input"], arguments);

    let removed = session
        .call("anima_record_link_remove", json!({ "id": ID }))
        .await;
    assert_eq!(structured(&removed)["removed"], ID);
    assert_eq!(sent(&hub, "RecordLinkRemove")[0], json!({ "id": ID }));
    session.finish().await;
}

const PLACE_ID: &str = "0190a000-0000-7000-9200-000000000001";

fn place() -> Value {
    json!({ "id": PLACE_ID, "parentId": null, "name": "Home", "title": "Home", "latitude": 52.0,
            "longitude": 4.0, "radiusM": 100, "category": "home", "icon": "house",
            "aliases": [], "geocoderName": null, "siteIds": [],
            "createdAt": "2026-11-18T10:00:00+00:00", "updatedAt": "2026-11-18T10:00:00+00:00" })
}

#[tokio::test]
async fn mcp_002_the_place_tools_that_change_are_off_until_writing_is_allowed() {
    let hub = hub().await;
    let mut reading = Desk::new().start(&hub, LOGIN_TOKEN, &[]).await.0;
    let names = reading.tool_names().await;
    assert!(names.iter().any(|name| name == "anima_places_list"));
    assert!(names.iter().any(|name| name == "anima_place_get"));
    for changing in [
        "anima_place_add",
        "anima_place_set",
        "anima_place_merge",
        "anima_place_split",
        "anima_place_remove",
        "anima_place_site",
    ] {
        assert!(!names.iter().any(|name| name == changing), "{changing}");
    }
    reading.finish().await;
}

#[tokio::test]
async fn mcp_017_the_place_tools_list_add_merge_and_remove() {
    let hub = hub().await;
    hub.answer("PlaceList", data(json!({ "places": [place()] })));
    hub.answer("PlaceAdd", data(json!({ "createPlace": place() })));
    hub.answer("PlaceMerge", data(json!({ "mergePlaces": place() })));
    hub.answer("PlaceRemove", data(json!({ "deletePlace": PLACE_ID })));
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;

    let listed = session.call("anima_places_list", json!({})).await;
    assert_eq!(structured(&listed)["places"][0]["name"], "Home");

    let arguments = json!({ "name": "Home", "category": "home", "icon": "house",
                            "latitude": 52.0, "longitude": 4.0, "radiusM": 100 });
    let added = session.call("anima_place_add", arguments.clone()).await;
    assert_eq!(structured(&added)["place"]["id"], PLACE_ID);
    assert_eq!(sent(&hub, "PlaceAdd")[0]["input"], arguments);

    let merged = session
        .call(
            "anima_place_merge",
            json!({ "from": "a", "into": PLACE_ID }),
        )
        .await;
    assert_eq!(structured(&merged)["place"]["id"], PLACE_ID);
    assert_eq!(
        sent(&hub, "PlaceMerge")[0],
        json!({ "from": "a", "into": PLACE_ID })
    );

    let removed = session
        .call(
            "anima_place_remove",
            json!({ "id": "a", "replacement": PLACE_ID }),
        )
        .await;
    assert_eq!(structured(&removed)["removed"], PLACE_ID);
    assert_eq!(
        sent(&hub, "PlaceRemove")[0],
        json!({ "id": "a", "replacement": PLACE_ID })
    );
    session.finish().await;
}
