//! Extraction proposals (VIS-001 to VIS-004), and what the audit log and the event log keep of
//! files and proposals (LOG-007).

use anima_hub::engine::{files, records::Reference};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{BOOK, FileHub};

const FIELDS: &str = "id owner form origin state decidedAt fields { name type value confidence }";

async fn propose(hub: &FileHub, origin: &str, extension: &str) -> Value {
    let input = json!({
        "owner": BOOK, "form": "book", "extension": extension, "origin": origin,
        "fields": [
            { "name": "title", "type": "TEXT", "value": "Emma PLAIN-VALUE-XYZ", "confidence": 0.8 },
            { "name": "published", "type": "DATE", "value": "1815-12-23", "confidence": null },
        ],
    });
    hub.post(
        &format!("mutation($input: ProposeExtractionInput!) {{ proposeExtraction(input: $input) {{ {FIELDS} }} }}"),
        json!({ "input": input }),
    )
    .await
}

async fn confirm(hub: &FileHub, id: &Value, title: &str) -> Value {
    let fields = json!([{ "name": "title", "type": "TEXT", "value": title, "confidence": null }]);
    hub.post(
        &format!("mutation($id: ID!, $fields: [ExtractedFieldInput!]!) {{ confirmExtraction(id: $id, fields: $fields) {{ {FIELDS} }} }}"),
        json!({ "id": id, "fields": fields }),
    )
    .await
}

async fn listed(hub: &FileHub, state: &str) -> Value {
    let answer = hub
        .post(
            &format!("query($owner: String, $state: ProposalState) {{ extractionProposals(owner: $owner, state: $state) {{ {FIELDS} }} }}"),
            json!({ "owner": BOOK, "state": state }),
        )
        .await;
    answer["data"]["extractionProposals"].clone()
}

#[sqlx::test(migrations = "./migrations")]
async fn vis_001_a_proposal_is_not_confirmed_until_the_operator_confirms_it(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let owner = Reference::parse(BOOK).unwrap();
    let proposed = propose(&hub, "CLIENT", "library").await;
    let proposed = &proposed["data"]["proposeExtraction"];
    assert_eq!(proposed["state"], "PROPOSED", "{proposed}");
    assert_eq!(
        proposed["fields"][1]["confidence"],
        Value::Null,
        "null stays null (VIS-004)"
    );
    assert_eq!(listed(&hub, "CONFIRMED").await, json!([]));
    assert_eq!(files::confirmed(&pool, &owner).await.unwrap().len(), 0);

    let confirmed = confirm(&hub, &proposed["id"], "Emma, as the operator wrote it").await;

    let confirmed = &confirmed["data"]["confirmExtraction"];
    assert_eq!(confirmed["state"], "CONFIRMED");
    assert_eq!(
        confirmed["fields"],
        json!([{ "name": "title", "type": "TEXT", "value": "Emma, as the operator wrote it", "confidence": null }])
    );
    let read = files::confirmed(&pool, &owner).await.unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].fields[0].value, "Emma, as the operator wrote it");
    assert_eq!(read[0].fields[0].confidence, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn vis_001_a_decided_proposal_does_not_change(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let proposed = propose(&hub, "CLIENT", "library").await;
    let id = proposed["data"]["proposeExtraction"]["id"].clone();
    let rejected = hub
        .post(
            &format!("mutation($id: ID!) {{ rejectExtraction(id: $id) {{ {FIELDS} }} }}"),
            json!({ "id": id }),
        )
        .await;
    assert_eq!(rejected["data"]["rejectExtraction"]["state"], "REJECTED");

    let confirmed = confirm(&hub, &id, "changed").await;
    let again = hub
        .post(
            &format!("mutation($id: ID!) {{ rejectExtraction(id: $id) {{ {FIELDS} }} }}"),
            json!({ "id": id }),
        )
        .await;

    assert_eq!(confirmed["errors"][0]["extensions"]["code"], "CONFLICT");
    assert_eq!(again["errors"][0]["extensions"]["code"], "CONFLICT");
    let stored = listed(&hub, "REJECTED").await;
    assert_eq!(stored[0]["fields"][0]["value"], "Emma PLAIN-VALUE-XYZ");
}

#[sqlx::test(migrations = "./migrations")]
async fn vis_002_the_hub_refuses_its_own_extraction_for_a_class_a_form(pool: PgPool) {
    let hub = FileHub::new(&pool).await;

    let refused = propose(&hub, "HUB", "documents").await;
    let allowed = propose(&hub, "HUB", "library").await;

    assert_eq!(refused["errors"][0]["extensions"]["code"], "VALIDATION");
    assert_eq!(
        refused["errors"][0]["extensions"]["fields"][0]["field"],
        "origin"
    );
    assert_eq!(allowed["data"]["proposeExtraction"]["origin"], "HUB");
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_no_byte_of_a_file_no_key_and_no_field_value_reaches_a_log(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let made = hub
        .upload(
            &format!("owner={BOOK}&name=n.txt&key=engine"),
            b"SECRET-CONTENT-123",
        )
        .await;
    hub.download(made["id"].as_str().unwrap()).await;
    let proposed = propose(&hub, "CLIENT", "library").await;
    confirm(
        &hub,
        &proposed["data"]["proposeExtraction"]["id"],
        "PLAIN-VALUE-XYZ again",
    )
    .await;
    let key = std::fs::read_to_string(hub.keys.path().join("engine.key")).unwrap();

    let audit: Vec<String> = sqlx::query_scalar("select change::text from events.audit_log")
        .fetch_all(&pool)
        .await
        .unwrap();
    let events: Vec<String> = sqlx::query_scalar("select message from events.event_log")
        .fetch_all(&pool)
        .await
        .unwrap();

    let written = [audit.join("\n"), events.join("\n")].join("\n");
    for secret in ["SECRET-CONTENT", "PLAIN-VALUE-XYZ", key.trim()] {
        assert!(!written.contains(secret), "{secret} in {written}");
    }
    let downloads: i64 = sqlx::query_scalar(
        "select count(*) from events.audit_log where target_type = 'attachment' and action = 'download'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(downloads, 1);
    assert!(
        audit
            .iter()
            .any(|change| change.contains(made["hash"].as_str().unwrap()))
    );
}
