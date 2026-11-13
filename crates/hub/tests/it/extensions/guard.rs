//! Both fields need a sign-in; `setExtension` is closed to API keys (API-003, D-23).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{SET, signed_in};
use crate::hub::{error, start, unauthenticated};

#[sqlx::test(migrations = "./migrations")]
async fn api_003_signed_out_requests_are_unauthenticated(pool: PgPool) {
    let (hub, _) = signed_in(&pool).await;
    let list = hub.post(None, "{ extensions { key } }", Value::Null).await;
    assert_eq!(error(&list), unauthenticated());
    let set = hub
        .post(None, SET, json!({ "key": "projects", "on": true }))
        .await;
    assert_eq!(error(&set), unauthenticated());
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_an_api_key_gets_forbidden_on_set_extension_and_nothing_changes(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let create = "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";
    let input = json!({ "name": "writer", "scope": "WRITE", "madeFor": "USER",
        "expiresAt": start() + chrono::TimeDelta::days(30) });
    let made = hub
        .post(Some(&token), create, json!({ "input": input }))
        .await;
    let secret = made["data"]["createApiKey"]["secret"].as_str().unwrap();

    let response = hub
        .post(Some(secret), SET, json!({ "key": "projects", "on": true }))
        .await;
    assert_eq!(error(&response).1, "FORBIDDEN", "{response}");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM extensions.switches")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    let read = hub
        .post(Some(secret), "{ extensions { key } }", Value::Null)
        .await;
    assert!(read["errors"].is_null(), "an API key may list: {read}");
}
