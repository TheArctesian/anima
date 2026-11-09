//! Creating, listing and deleting channels through the API: each kind reads back with its
//! settings and never a secret (NOT-002, NOT-003, SEC-003, SEC-005, D-17, CR-14).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    DELETE, WEBHOOK_URL, channels, create, create_query, data, error_json, holds_no_secret,
    instant, now, run, schema_for, stored_row, validation_fields, webhook,
};

#[sqlx::test(migrations = "./migrations")]
async fn sec_005_a_webhook_reads_back_with_its_origin_and_header_names_only(pool: PgPool) {
    let schema = schema_for(&pool);
    let created = create(&schema, webhook("Ops hook")).await;
    holds_no_secret("the create answer", &created.to_string());
    assert_eq!(created["name"], "Ops hook");
    assert_eq!(created["kind"], "WEBHOOK");
    assert_eq!(created["onByDefault"], false);
    assert_eq!(created["template"], Value::Null);
    assert_eq!(created["lastDelivery"], Value::Null);
    assert_eq!(
        created["settings"],
        json!({
            "__typename": "WebhookSettings",
            "origin": "https://hooks.example.test",
            "method": "PUT",
            "headerNames": ["X-Api-Key"],
        })
    );
    assert_eq!(instant(&created["createdAt"]), now());
    let listed = channels(&schema).await;
    assert_eq!(listed, std::slice::from_ref(&created));
    let (row, sealed) = stored_row(&pool, &created["id"]).await;
    holds_no_secret("the stored settings", &row);
    holds_no_secret("the sealed secrets", &String::from_utf8_lossy(&sealed));
    assert_ne!(sealed, Vec::<u8>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_each_kind_is_created_with_the_k1_defaults(pool: PgPool) {
    let schema = schema_for(&pool);
    let ntfy = create(
        &schema,
        json!({ "name": "Phone", "onByDefault": true,
                "settings": { "ntfy": { "topic": "topic-s3cr3t", "token": "tk_s3cr3t" } } }),
    )
    .await;
    assert_eq!(ntfy["onByDefault"], true);
    assert_eq!(
        ntfy["settings"],
        json!({ "__typename": "NtfySettings", "server": "https://ntfy.sh", "priority": 3,
                "tokenStored": true })
    );
    let email = create(
        &schema,
        json!({ "name": "Mail", "template": { "title": "{{check}}", "body": "{{status}}" },
                "settings": { "email": { "host": "smtp.example.test", "username": "hub",
                  "password": "smtp-s3cr3t", "from": "Anima <hub@example.test>",
                  "to": ["ops@example.test"] } } }),
    )
    .await;
    assert_eq!(
        email["template"],
        json!({ "title": "{{check}}", "body": "{{status}}" })
    );
    assert_eq!(
        email["settings"],
        json!({ "__typename": "EmailSettings", "host": "smtp.example.test", "port": 587,
                "security": "STARTTLS", "username": "hub", "from": "Anima <hub@example.test>",
                "to": ["ops@example.test"], "passwordStored": true })
    );
    let apprise = create(
        &schema,
        json!({ "name": "Apprise", "settings": { "apprise": {
            "server": "https://apprise.example.test",
            "urls": ["discord://1234/apprise-s3cr3t?format=markdown"] } } }),
    )
    .await;
    assert_eq!(
        apprise["settings"],
        json!({ "__typename": "AppriseSettings", "server": "https://apprise.example.test",
                "mode": "STATELESS_URLS", "tags": [] })
    );
    let listed = channels(&schema).await;
    let names: Vec<&str> = listed.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Apprise", "Mail", "Phone"]);
    holds_no_secret("the channel list", &Value::Array(listed).to_string());
}

/// An explicit null takes the K1 default, as an absent field does.
#[sqlx::test(migrations = "./migrations")]
async fn not_002_an_explicit_null_takes_the_k1_default(pool: PgPool) {
    let schema = schema_for(&pool);
    let hook = create(
        &schema,
        json!({ "name": "Hook", "onByDefault": null, "settings": { "webhook": {
            "url": WEBHOOK_URL, "method": null } } }),
    )
    .await;
    assert_eq!(hook["onByDefault"], false);
    assert_eq!(hook["settings"]["method"], "POST");
    let ntfy = create(
        &schema,
        json!({ "name": "Phone", "settings": { "ntfy": {
            "server": null, "topic": "topic-s3cr3t", "priority": null } } }),
    )
    .await;
    assert_eq!(ntfy["settings"]["server"], "https://ntfy.sh");
    assert_eq!(ntfy["settings"]["priority"], 3);
    let email = create(
        &schema,
        json!({ "name": "Mail", "settings": { "email": { "host": "smtp.example.test",
            "port": null, "security": null, "from": "hub@example.test",
            "to": ["ops@example.test"] } } }),
    )
    .await;
    assert_eq!(email["settings"]["port"], 587);
    assert_eq!(email["settings"]["security"], "STARTTLS");
    let apprise = create(
        &schema,
        json!({ "name": "Apprise", "settings": { "apprise": {
            "server": "https://apprise.example.test", "mode": null, "tags": null,
            "urls": ["discord://1234/apprise-s3cr3t"] } } }),
    )
    .await;
    assert_eq!(apprise["settings"]["mode"], "STATELESS_URLS");
    assert_eq!(apprise["settings"]["tags"], json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_a_taken_name_is_a_validation_error_on_name(pool: PgPool) {
    let schema = schema_for(&pool);
    create(&schema, webhook("Ops hook")).await;
    let again = run(
        &schema,
        &create_query(),
        json!({ "input": webhook("Ops hook") }),
    )
    .await;
    assert_eq!(validation_fields(&again), ["name"]);
    assert_eq!(channels(&schema).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn cr_14_6_an_invalid_channel_is_refused_by_field_and_nothing_is_stored(pool: PgPool) {
    let schema = schema_for(&pool);
    let input = json!({ "name": "", "settings": { "webhook": {
        "url": "http://hooks.example.test/s3cr3t-path",
        "headers": [{ "name": "Host", "value": "header-s3cr3t" }] } } });
    let response = run(&schema, &create_query(), json!({ "input": input })).await;
    assert_eq!(
        validation_fields(&response),
        ["name", "settings.webhook.url", "settings.webhook.headers"]
    );
    holds_no_secret("the refusal", &error_json(&response).to_string());
    assert_eq!(channels(&schema).await, Vec::<Value>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_an_input_that_cannot_be_read_is_refused_without_quoting_it(pool: PgPool) {
    let schema = schema_for(&pool);
    let shapes = [
        json!({ "name": "x", "settings": { "webhook": { "url": ["s3cr3t-path"] } } }),
        json!({ "name": "x", "settings": { "ntfy": { "topic": 1, "token": "tk_s3cr3t" } } }),
        json!({ "name": "x", "settings": { "webhook": { "url": WEBHOOK_URL },
                                           "ntfy": { "topic": "topic-s3cr3t" } } }),
        json!({ "name": "x", "settings": { "email": { "password": "smtp-s3cr3t" } } }),
    ];
    for input in shapes {
        let response = run(&schema, &create_query(), json!({ "input": input })).await;
        assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
        holds_no_secret("the parse error", &format!("{:?}", response.errors));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_delete_removes_the_channel_and_a_second_delete_finds_nothing(pool: PgPool) {
    let schema = schema_for(&pool);
    let created = create(&schema, webhook("Ops hook")).await;
    let deleted = data(&schema, DELETE, json!({ "id": created["id"] })).await;
    assert_eq!(deleted["deleteNotificationChannel"], created["id"]);
    assert_eq!(channels(&schema).await, Vec::<Value>::new());
    let again = run(&schema, DELETE, json!({ "id": created["id"] })).await;
    assert_eq!(error_json(&again)["extensions"]["code"], "NOT_FOUND");
    let bad = run(&schema, DELETE, json!({ "id": "not-an-id" })).await;
    assert_eq!(validation_fields(&bad), ["id"]);
}
