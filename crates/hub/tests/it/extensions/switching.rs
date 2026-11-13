//! `setExtension`: on, off, and the refusals (EXT-001, EXT-003, EXT-011, EXT-012).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{LIST, set, signed_in};
use crate::hub::error;

#[sqlx::test(migrations = "./migrations")]
async fn ext_011_on_returns_what_changed_and_the_list_then_shows_it_on(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let response = set(&hub, &token, "projects", true).await;
    let changed = response["data"]["setExtension"].as_array().unwrap();
    assert_eq!(changed.len(), 1, "{response}");
    assert_eq!(
        (&changed[0]["key"], &changed[0]["on"]),
        (&json!("projects"), &json!(true))
    );
    assert_ne!(changed[0]["changedAt"], Value::Null);
    let again = set(&hub, &token, "projects", true).await;
    assert_eq!(again["data"]["setExtension"], json!([]), "nothing changed");

    let listed = hub.post(Some(&token), LIST, Value::Null).await;
    let on: Vec<_> = listed["data"]["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["on"] == true)
        .map(|e| e["key"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(on, ["projects", "integrations"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_003_off_keeps_the_row_and_turns_the_switch_off(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    set(&hub, &token, "projects", true).await;
    let response = set(&hub, &token, "projects", false).await;
    assert_eq!(
        response["data"]["setExtension"][0]["on"], false,
        "{response}"
    );
    let stored: Vec<(String, bool)> = sqlx::query_as("SELECT key, is_on FROM extensions.switches")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(stored, [("projects".to_owned(), false)]);
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_011_an_unbuilt_extension_and_the_fixed_one_are_conflict_and_change_nothing(
    pool: PgPool,
) {
    let (hub, token) = signed_in(&pool).await;
    for (key, on) in [
        ("map", true),
        ("model_gateway", true),
        ("map", false),
        ("integrations", false),
    ] {
        let response = set(&hub, &token, key, on).await;
        assert_eq!(error(&response).1, "CONFLICT", "{key} {on}: {response}");
    }
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM extensions.switches")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn ext_011_an_unknown_key_is_a_validation_error(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let response = set(&hub, &token, "nothing", true).await;
    assert_eq!(error(&response).1, "VALIDATION", "{response}");
}
