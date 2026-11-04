//! Tokens at the edge: storage, expiry, logout, `me` and API-003 for every root field
//! (SEC-002, API-003, LOG-007, D-7, D-9).

use anima_hub::access::{self, Password};
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
    let health = hub.post(Some(token), "{ health }", json!({})).await;
    assert_eq!(health, json!({ "data": { "health": "ok" } }));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_7_token_is_stored_only_as_its_hash(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let login = hub.login(USERNAME, PASSWORD).await;
    let token = login["data"]["login"]["token"].as_str().unwrap();
    assert_eq!(
        login["data"]["login"]["expiresAt"],
        "2030-06-04T12:00:00+00:00"
    );

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

#[sqlx::test(migrations = "./migrations")]
async fn api_003_connection_init_payload_identifies_a_websocket(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let payload = json!({ "token": token });
    let identity = access::identify_connection(&pool, &payload, start())
        .await
        .unwrap();
    assert_eq!(
        identity.map(|i| i.operator().name.clone()),
        Some(USERNAME.to_owned())
    );
    let missing = access::identify_connection(&pool, &json!({}), start())
        .await
        .unwrap();
    assert!(missing.is_none());
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
async fn d_9_operator_set_replaces_the_password_and_revokes_tokens(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let replaced = "a different long passphrase";
    let id = access::set_operator(&pool, USERNAME, Password::new(replaced.to_owned()), start())
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar("select count(*) from access.operators")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        error(&hub.post(Some(&token), ME, json!({})).await),
        unauthenticated()
    );
    assert!(hub.login(USERNAME, PASSWORD).await["errors"].is_array());
    let response = hub.login(USERNAME, replaced).await;
    assert_eq!(response["data"]["login"]["operator"]["id"], id.to_string());

    let short = access::set_operator(&pool, USERNAME, Password::new("short".to_owned()), start());
    assert!(matches!(
        short.await,
        Err(access::Error::Validation {
            field: "password",
            ..
        })
    ));
    let spaced = access::set_operator(
        &pool,
        "an operator",
        Password::new(replaced.to_owned()),
        start(),
    );
    assert!(matches!(
        spaced.await,
        Err(access::Error::Validation {
            field: "username",
            ..
        })
    ));
}
