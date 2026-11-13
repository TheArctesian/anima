//! `extensions` lists the registry (EXT-001).

use serde_json::Value;
use sqlx::PgPool;

use super::{LIST, signed_in};

#[sqlx::test(migrations = "./migrations")]
async fn ext_001_a_new_database_lists_fifteen_and_only_the_fixed_one_is_on(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let response = hub.post(Some(&token), LIST, Value::Null).await;
    let listed = response["data"]["extensions"].as_array().unwrap();
    assert_eq!(listed.len(), 15, "{response}");
    let on: Vec<_> = listed
        .iter()
        .filter(|e| e["on"] == true)
        .map(|e| e["key"].as_str().unwrap())
        .collect();
    assert_eq!(on, ["integrations"]);
    let projects = listed.iter().find(|e| e["key"] == "projects").unwrap();
    assert_eq!(projects["built"], true);
    assert_eq!(projects["navigation"]["route"], "/projects");
    assert_eq!(projects["dataClass"], "C");
    assert_eq!(projects["changedAt"], Value::Null);
    let gateway = listed.iter().find(|e| e["key"] == "model_gateway").unwrap();
    assert_eq!(gateway["needs"][0], "secrets");
    assert_eq!(gateway["built"], false);
}
