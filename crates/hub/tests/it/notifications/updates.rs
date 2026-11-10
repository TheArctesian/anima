//! The CR-14 update rules through the API: null keeps, a new target needs the secrets again,
//! the kind never changes, and stored secrets that do not open are asked for again.

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    create, error_json, holds_no_secret, schema_for, stored_row, update, validation_fields, webhook,
};

fn webhook_change(url: &Value, headers: &Value) -> Value {
    json!({ "name": "Ops hook", "settings": { "webhook": { "url": url, "headers": headers } } })
}

#[sqlx::test(migrations = "./migrations")]
async fn cr_14_1_null_secrets_keep_the_stored_ones_and_the_settings_change(pool: PgPool) {
    let schema = schema_for(&pool);
    let created = create(&schema, webhook("Ops hook")).await;
    let response = update(
        &schema,
        &created["id"],
        webhook_change(&Value::Null, &Value::Null),
    )
    .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let updated = response.data.into_json().unwrap()["updateNotificationChannel"].clone();
    assert_eq!(updated["settings"]["headerNames"], json!(["X-Api-Key"]));
    assert_eq!(updated["settings"]["method"], "POST");
    assert_eq!(updated["createdAt"], created["createdAt"]);
    holds_no_secret("the update answer", &updated.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn cr_14_1_a_new_origin_with_kept_headers_is_refused(pool: PgPool) {
    let schema = schema_for(&pool);
    let created = create(&schema, webhook("Ops hook")).await;
    let before = stored_row(&pool, &created["id"]).await;
    let elsewhere = json!("https://elsewhere.example.test/hook");
    let response = update(
        &schema,
        &created["id"],
        webhook_change(&elsewhere, &Value::Null),
    )
    .await;
    assert_eq!(validation_fields(&response), ["settings.webhook.headers"]);
    assert_eq!(stored_row(&pool, &created["id"]).await, before);
    let cleared = update(
        &schema,
        &created["id"],
        webhook_change(&elsewhere, &json!([])),
    )
    .await;
    assert!(cleared.errors.is_empty(), "{:?}", cleared.errors);
    let settings = &cleared.data.into_json().unwrap()["updateNotificationChannel"]["settings"];
    assert_eq!(settings["origin"], "https://elsewhere.example.test");
    assert_eq!(settings["headerNames"], json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn cr_14_4_a_kind_change_is_refused(pool: PgPool) {
    let schema = schema_for(&pool);
    let created = create(&schema, webhook("Ops hook")).await;
    let ntfy = json!({ "name": "Ops hook", "settings": { "ntfy": { "topic": "topic-s3cr3t" } } });
    let response = update(&schema, &created["id"], ntfy).await;
    assert_eq!(validation_fields(&response), ["settings"]);
    let error = error_json(&response);
    assert_eq!(
        error["extensions"]["fields"][0]["message"],
        "A channel's kind cannot change. Create a new channel."
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_003_secrets_copied_from_another_channel_do_not_open(pool: PgPool) {
    let schema = schema_for(&pool);
    let first = create(&schema, webhook("First")).await;
    let second = create(&schema, webhook("Second")).await;
    sqlx::query(
        "UPDATE notifications.channels SET secrets = \
         (SELECT secrets FROM notifications.channels WHERE id = $1) WHERE id = $2",
    )
    .bind(first["id"].as_str().unwrap().parse::<uuid::Uuid>().unwrap())
    .bind(
        second["id"]
            .as_str()
            .unwrap()
            .parse::<uuid::Uuid>()
            .unwrap(),
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut kept = webhook_change(&Value::Null, &Value::Null);
    kept["name"] = json!("Second");
    let response = update(&schema, &second["id"], kept).await;
    assert_eq!(validation_fields(&response), ["settings"]);
    assert_eq!(
        error_json(&response)["extensions"]["fields"][0]["message"],
        "The stored secrets cannot be opened. Enter every secret again."
    );
    let response = update(&schema, &second["id"], webhook("Second")).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_an_empty_ntfy_token_clears_it_and_a_missing_channel_is_not_found(pool: PgPool) {
    let schema = schema_for(&pool);
    let input = json!({ "name": "Phone",
        "settings": { "ntfy": { "topic": "topic-s3cr3t", "token": "tk_s3cr3t" } } });
    let created = create(&schema, input).await;
    let cleared = json!({ "name": "Phone", "settings": { "ntfy": { "token": "" } } });
    let response = update(&schema, &created["id"], cleared.clone()).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let settings = &response.data.into_json().unwrap()["updateNotificationChannel"]["settings"];
    assert_eq!(settings["tokenStored"], false);
    let missing = json!("0190a000-0000-7000-8fff-000000000001");
    let response = update(&schema, &missing, cleared).await;
    assert_eq!(error_json(&response)["extensions"]["code"], "NOT_FOUND");
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_an_update_to_a_taken_name_is_a_validation_error_on_name(pool: PgPool) {
    let schema = schema_for(&pool);
    create(&schema, webhook("First")).await;
    let second = create(&schema, webhook("Second")).await;
    let before = stored_row(&pool, &second["id"]).await;
    let mut renamed = webhook_change(&Value::Null, &Value::Null);
    renamed["name"] = json!("First");
    let response = update(&schema, &second["id"], renamed).await;
    assert_eq!(validation_fields(&response), ["name"]);
    assert_eq!(
        error_json(&response)["extensions"]["fields"][0]["message"],
        "Another channel has this name."
    );
    assert_eq!(stored_row(&pool, &second["id"]).await, before);
}
