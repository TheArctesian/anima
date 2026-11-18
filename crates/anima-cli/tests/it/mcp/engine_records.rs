//! The `timeline` and `records` tool sets against the scripted hub (MCP-002, MCP-017;
//! TML-007, RLK-001, RLK-002), with the file and extraction tools of `records` (FIL-001,
//! FIL-004, VIS-001, VIS-004). No tool carries the bytes of a file (CMD-030).

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

const FILE_ID: &str = "0190a000-0000-7000-9100-000000000002";

fn proposal(state: &str, confidence: Value) -> Value {
    json!({ "id": FILE_ID, "attachmentId": null, "owner": "book:1", "form": "book",
            "origin": "CLIENT", "state": state, "createdAt": "2026-11-18T10:00:00+00:00",
            "decidedAt": null,
            "fields": [{ "name": "title", "type": "TEXT", "value": "Emma",
                         "confidence": confidence }] })
}

#[tokio::test]
async fn cmd_030_no_tool_uploads_or_downloads_the_bytes_of_a_file() {
    let hub = hub().await;
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let names = session.tool_names().await;
    for name in [
        "anima_files_list",
        "anima_file_remove",
        "anima_file_archive_add",
        "anima_file_archive_remove",
        "anima_extractions_list",
        "anima_extraction_propose",
        "anima_extraction_confirm",
        "anima_extraction_reject",
    ] {
        assert!(names.iter().any(|listed| listed == name), "{name}");
    }
    assert!(
        !names
            .iter()
            .any(|name| name.contains("upload") || name.contains("download"))
    );
    session.finish().await;
}

#[tokio::test]
async fn mcp_017_the_file_tools_list_and_remove_a_file() {
    let hub = hub().await;
    let file = json!({ "id": FILE_ID, "owner": "book:1", "name": "cover.jpg",
        "mediaType": "image/jpeg", "hash": "ab", "key": "engine", "size": 10,
        "archiveId": null, "latitude": null, "longitude": null,
        "createdAt": "2026-11-18T10:00:00+00:00" });
    hub.answer("AttachmentList", data(json!({ "attachments": [file] })));
    hub.answer(
        "AttachmentRemove",
        data(json!({ "deleteAttachment": FILE_ID })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;

    let listed = session
        .call("anima_files_list", json!({ "owner": "book:1" }))
        .await;
    let removed = session
        .call("anima_file_remove", json!({ "id": FILE_ID }))
        .await;

    assert_eq!(structured(&listed)["files"][0]["name"], "cover.jpg");
    assert_eq!(structured(&removed)["removed"], FILE_ID);
    assert_eq!(sent(&hub, "AttachmentRemove")[0], json!({ "id": FILE_ID }));
    session.finish().await;
}

#[tokio::test]
async fn vis_004_a_proposal_without_a_confidence_is_sent_and_read_as_null() {
    let hub = hub().await;
    hub.answer(
        "ExtractionPropose",
        data(json!({ "proposeExtraction": proposal("PROPOSED", Value::Null) })),
    );
    hub.answer(
        "ExtractionConfirm",
        data(json!({ "confirmExtraction": proposal("CONFIRMED", Value::Null) })),
    );
    let desk = Desk::new();
    let mut session = writing(&hub, &desk).await;
    let fields = json!([{ "name": "title", "type": "TEXT", "value": "Emma" }]);

    let proposed = session
        .call(
            "anima_extraction_propose",
            json!({ "owner": "book:1", "form": "book",
            "extension": "library", "origin": "CLIENT", "fields": fields }),
        )
        .await;
    let confirmed = session
        .call(
            "anima_extraction_confirm",
            json!({ "id": FILE_ID, "fields": fields }),
        )
        .await;

    assert_eq!(
        structured(&proposed)["proposal"]["fields"][0]["confidence"],
        Value::Null
    );
    assert_eq!(structured(&confirmed)["proposal"]["state"], "CONFIRMED");
    let input = &sent(&hub, "ExtractionPropose")[0]["input"];
    assert_eq!(input["fields"][0]["confidence"], Value::Null);
    assert_eq!(input["origin"], "CLIENT");
    assert_eq!(
        sent(&hub, "ExtractionConfirm")[0]["fields"][0]["type"],
        "TEXT"
    );
    session.finish().await;
}
