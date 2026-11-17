//! `anima record-link list`, `add` and `remove` against the scripted hub (RLK-001, RLK-002).

use serde_json::{Value, json};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

const PERSON: &str = "person:0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d";
const ID: &str = "0190a000-0000-7000-9100-000000000001";

fn signed_in() -> Anima {
    Anima::new().with("ANIMA_TOKEN", "login-token-for-link-tests")
}

fn link(label: Option<&str>) -> Value {
    json!({
        "id": ID, "source": "book:1", "target": PERSON, "kind": "lent_to", "label": label,
        "sourceTitle": null, "targetTitle": null, "createdAt": "2026-11-16T10:00:00+00:00",
    })
}

#[tokio::test]
async fn rlk_002_record_link_list_shows_the_label_of_a_target_with_no_record() {
    let hub = FakeHub::start().await;
    hub.answer(
        "RecordLinkList",
        data(json!({ "recordLinks": [link(Some("Ada Lovelace"))] })),
    );
    let run = signed_in()
        .run(&["record-link", "list", "book:1", "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    let row = run
        .stdout
        .lines()
        .find(|line| line.starts_with("book:1"))
        .expect("a row");
    assert!(row.contains("lent_to"), "{row}");
    assert!(row.contains(&format!("Ada Lovelace ({PERSON})")), "{row}");
    assert_eq!(hub.seen()[0].variables, json!({ "record": "book:1" }));

    let run = signed_in()
        .run(&[
            "record-link",
            "list",
            "book:1",
            "--json",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.json(), json!([link(Some("Ada Lovelace"))]));
}

#[tokio::test]
async fn rlk_001_record_link_add_sends_the_link_and_prints_it() {
    let hub = FakeHub::start().await;
    hub.answer(
        "RecordLinkAdd",
        data(json!({ "linkRecords": link(Some("Ada")) })),
    );
    let run = signed_in()
        .run(&[
            "record-link",
            "add",
            "book:1",
            PERSON,
            "--kind",
            "lent_to",
            "--label",
            "Ada",
            "--hub",
            &hub.origin,
        ])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains(ID) && run.stdout.contains("Ada"),
        "{}",
        run.stdout
    );
    assert_eq!(
        hub.seen()[0].variables["input"],
        json!({ "source": "book:1", "target": PERSON, "kind": "lent_to", "label": "Ada" })
    );

    let refusing = FakeHub::start().await;
    refusing.answer(
        "RecordLinkAdd",
        refusal("VALIDATION", "Some input is not valid."),
    );
    let run = signed_in()
        .run(&[
            "record-link",
            "add",
            "book",
            PERSON,
            "--kind",
            "lent_to",
            "--json",
            "--hub",
            &refusing.origin,
        ])
        .await;
    assert_ne!(run.code, 0);
    assert_eq!(run.error()["code"], "VALIDATION", "{run:?}");
    let sent = &refusing.seen()[0].variables["input"];
    assert!(sent.get("label").is_none_or(Value::is_null), "{sent}");
}

#[tokio::test]
async fn rlk_001_record_link_remove_says_what_it_removed_and_an_unknown_id_fails() {
    let hub = FakeHub::start().await;
    hub.answer("RecordLinkRemove", data(json!({ "unlinkRecords": ID })));
    let run = signed_in()
        .run(&["record-link", "remove", ID, "--hub", &hub.origin])
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert!(
        run.stdout.contains(&format!("Removed the link {ID}.")),
        "{}",
        run.stdout
    );
    assert_eq!(hub.seen()[0].variables, json!({ "id": ID }));

    let refusing = FakeHub::start().await;
    refusing.answer(
        "RecordLinkRemove",
        refusal("NOT_FOUND", "No record link has this ID."),
    );
    let run = signed_in()
        .run(&[
            "record-link",
            "remove",
            ID,
            "--json",
            "--hub",
            &refusing.origin,
        ])
        .await;
    assert_ne!(run.code, 0);
    assert_eq!(run.error()["code"], "NOT_FOUND", "{run:?}");
}
