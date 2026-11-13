//! Tokens at the edge: storage, expiry, logout, `me` and API-003 for every root field
//! (SEC-002, API-003, LOG-007, D-7, D-9).

use anima_hub::fleet::access::{self, Password};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::TimeDelta;
use serde_json::json;
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use crate::hub::{Hub, PASSWORD, USERNAME, error, set_operator, start, unauthenticated};

const ME: &str = "{ me { id username totpEnabled } }";

#[sqlx::test(migrations = "./migrations")]
async fn api_003_me_answers_with_a_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.login(USERNAME, PASSWORD).await;
    let token = login["data"]["login"]["token"].as_str().unwrap();
    let response = hub.post(Some(token), ME, json!({})).await;
    assert_eq!(response["data"]["me"], login["data"]["login"]["operator"]);
    assert_eq!(response["data"]["me"]["username"], USERNAME);
    assert_eq!(response["data"]["me"]["totpEnabled"], false);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_7_token_is_stored_only_as_its_hash(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.login(USERNAME, PASSWORD).await;
    let token = login["data"]["login"]["token"].as_str().unwrap();
    // R-033: compare instants, not text.
    let expires_at: chrono::DateTime<chrono::Utc> = login["data"]["login"]["expiresAt"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(expires_at, start() + TimeDelta::days(30));

    let rows: Vec<String> = sqlx::query_scalar(
        "select row_to_json(t)::text from access.api_tokens t \
         union all select row_to_json(o)::text from access.operators o \
         union all select row_to_json(a)::text from access.login_attempts a",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(rows.iter().all(|row| !row.contains(token)), "{rows:?}");
    let (hash, scope): (Vec<u8>, String) =
        sqlx::query_as("select token_hash, scope from access.api_tokens")
            .fetch_one(&pool)
            .await
            .unwrap();
    let bytes = URL_SAFE_NO_PAD.decode(token).unwrap();
    assert_eq!(hash, Sha256::digest(&bytes).to_vec());
    assert_eq!(scope, "write");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_an_expired_token_is_refused(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let last_moment = Hub::new(&pool, start() + TimeDelta::days(30) - TimeDelta::seconds(1));
    assert!(last_moment.post(Some(&token), ME, json!({})).await["data"]["me"].is_object());

    let expired = Hub::new(&pool, start() + TimeDelta::days(30));
    let response = expired.post(Some(&token), ME, json!({})).await;
    assert_eq!(error(&response), unauthenticated());
    let tunnel = access::operator_for_token(&pool, &token, start() + TimeDelta::days(30)).await;
    assert!(matches!(tunnel, Err(access::Error::Unauthenticated)));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_logout_revokes_the_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let other = hub.token().await;
    let logout = hub
        .post(Some(&token), "mutation { logout }", json!({}))
        .await;
    assert_eq!(logout, json!({ "data": { "logout": true } }));

    assert_eq!(
        error(&hub.post(Some(&token), ME, json!({})).await),
        unauthenticated()
    );
    let again = hub
        .post(Some(&token), "mutation { logout }", json!({}))
        .await;
    assert_eq!(error(&again), unauthenticated());
    assert!(hub.post(Some(&other), ME, json!({})).await["data"]["me"].is_object());
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_operator_for_token_resolves_and_touches_once_a_minute(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let last_used = || sqlx::query_scalar("select last_used_at from access.api_tokens");

    let operator = access::operator_for_token(&pool, &token, start())
        .await
        .unwrap();
    assert_eq!(operator.name, USERNAME);
    let at = start() + TimeDelta::seconds(30);
    access::operator_for_token(&pool, &token, at).await.unwrap();
    let first: Option<chrono::DateTime<chrono::Utc>> = last_used().fetch_one(&pool).await.unwrap();
    assert_eq!(first, Some(start()));
    access::operator_for_token(&pool, &token, start() + TimeDelta::minutes(1))
        .await
        .unwrap();
    let second: Option<chrono::DateTime<chrono::Utc>> = last_used().fetch_one(&pool).await.unwrap();
    assert_eq!(second, Some(start() + TimeDelta::minutes(1)));

    let junk = access::operator_for_token(&pool, "not a token", start()).await;
    assert!(matches!(junk, Err(access::Error::Unauthenticated)));
}

/// The root field names of the hub schema, per operation type, by introspection.
async fn root_fields(hub: &Hub, token: &str) -> Vec<(&'static str, String)> {
    let query = "{ __schema { queryType { fields { name } } mutationType { fields { name } } } }";
    let schema = &hub.post(Some(token), query, json!({})).await["data"]["__schema"];
    let mut fields = Vec::new();
    for (keyword, root) in [("query", "queryType"), ("mutation", "mutationType")] {
        for field in schema[root]["fields"].as_array().unwrap() {
            fields.push((keyword, field["name"].as_str().unwrap().to_owned()));
        }
    }
    fields
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_every_root_field_except_login_is_refused_without_a_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let fields = root_fields(&hub, &hub.token().await).await;
    assert!(fields.len() >= 4, "{fields:?}");
    let mut documents: Vec<String> = fields
        .iter()
        .filter(|(_, name)| name != "login")
        .map(|(keyword, name)| format!("{keyword} {{ {name} }}"))
        .collect();
    documents.push("{ __schema { queryType { name } } }".to_owned());
    for document in &documents {
        for token in [None, Some("garbage"), Some("A".repeat(43).as_str())] {
            let response = hub.post(token, document, json!({})).await;
            assert_eq!(error(&response), unauthenticated(), "{document}");
            assert_eq!(response["data"], json!(null), "{document}");
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_a_token_never_appears_in_a_response_error(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let expired = Hub::new(&pool, start() + TimeDelta::days(31));
    for query in [ME, "mutation { logout }", "{ nonsense }"] {
        let text = expired.post_text(Some(&token), query, json!({})).await;
        assert!(text.contains("errors"), "{text}");
        assert!(!text.contains(&token), "{text}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_9_operator_set_replaces_the_one_operator_and_revokes_tokens(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let first_id: uuid::Uuid = sqlx::query_scalar("select id from access.operators")
        .fetch_one(&pool)
        .await
        .unwrap();
    let replaced = "a different long passphrase";
    let id = access::set_operator(
        &pool,
        "newname",
        Password::new(replaced.to_owned()),
        start(),
    )
    .await
    .unwrap();

    let names: Vec<String> = sqlx::query_scalar("select username from access.operators")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(names, ["newname"], "exactly one operator (OD-2)");
    assert_eq!(
        id.to_string(),
        first_id.to_string(),
        "the account is replaced, not added"
    );
    let me = hub.post(Some(&token), ME, json!({})).await;
    assert_eq!(error(&me), unauthenticated());
    assert!(hub.login(USERNAME, PASSWORD).await["errors"].is_array());
    assert!(hub.login(USERNAME, replaced).await["errors"].is_array());
    let response = hub.login("newname", replaced).await;
    assert_eq!(response["data"]["login"]["operator"]["id"], id.to_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_9_the_database_refuses_a_second_operator(pool: PgPool) {
    set_operator(&pool).await;
    let second = sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values (gen_random_uuid(), 'second', 'x', now())",
    )
    .execute(&pool)
    .await;
    let constraint = second
        .unwrap_err()
        .as_database_error()
        .and_then(|e| e.constraint().map(str::to_owned));
    assert_eq!(constraint.as_deref(), Some("operators_single"));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_prune_deletes_old_attempts_and_expired_tokens(pool: PgPool) {
    set_operator(&pool).await;
    let old = Hub::new(&pool, start());
    old.login(USERNAME, "wrong password").await;
    let expired = old.token().await;
    let later = start() + TimeDelta::days(91);
    let current = Hub::new(&pool, later);
    current.login(USERNAME, "wrong password").await;
    let fresh = current.token().await;

    let pruned = access::prune(&pool, later).await.unwrap();
    assert_eq!(
        pruned,
        access::Pruned {
            attempts: 2,
            tokens: 1
        }
    );
    let left: i64 = sqlx::query_scalar("select count(*) from access.login_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 2);
    assert!(
        access::operator_for_token(&pool, &expired, start())
            .await
            .is_err()
    );
    assert!(
        access::operator_for_token(&pool, &fresh, later)
            .await
            .is_ok()
    );
}
