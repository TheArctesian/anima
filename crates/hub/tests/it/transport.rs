//! Where a token may travel: the `Authorization` header and the WebSocket `connection_init`
//! payload, never a URL query (K7 "Authentication transport", R-020, SEC-002).

use anima_hub::fleet::access;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use crate::hub::{Hub, USERNAME, set_operator, start};

const ME: &str = r#"{"query":"{ me { username } }"}"#;

fn me(uri: &str, authorization: Option<&str>) -> Request<Body> {
    let mut request = Request::post(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    request.body(Body::from(ME)).unwrap()
}

async fn answer(hub: &Hub, request: Request<Body>) -> Value {
    let (status, text) = hub.send(request).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    serde_json::from_str(&text).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_r_020_http_takes_the_token_only_from_a_bearer_header(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;

    let valid = answer(&hub, me("/graphql", Some(&format!("Bearer {token}")))).await;
    assert_eq!(valid["data"]["me"]["username"], USERNAME);
    for refused in [
        me("/graphql", None),
        me("/graphql", Some(&format!("Basic {token}"))),
        me("/graphql", Some(&token)),
        me(&format!("/graphql?token={token}"), None),
        me(&format!("/graphql?authorization=Bearer%20{token}"), None),
    ] {
        let response = answer(&hub, refused).await;
        assert_eq!(
            response["errors"][0]["extensions"]["code"], "UNAUTHENTICATED",
            "{response}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_r_020_connection_init_takes_a_bearer_authorization(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let identify = |payload: Value| {
        let pool = pool.clone();
        async move {
            access::identify_connection(&pool, &payload, start())
                .await
                .unwrap()
                .and_then(|identity| identity.operator().map(|operator| operator.name.clone()))
        }
    };
    let bearer = json!({ "authorization": format!("Bearer {token}") });
    assert_eq!(identify(bearer).await.as_deref(), Some(USERNAME));
    let lower = json!({ "authorization": format!("bearer {token}") });
    assert_eq!(identify(lower).await.as_deref(), Some(USERNAME));
    for refused in [
        json!({ "token": token }),
        json!({ "authorization": format!("Basic {token}") }),
        json!({ "authorization": token }),
        json!({ "authorization": 42 }),
        json!({ "authorization": [format!("Bearer {token}")] }),
        json!({}),
        json!(null),
    ] {
        assert_eq!(identify(refused.clone()).await, None, "{refused}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_read_key_may_read_but_not_sign_out_on_the_real_schema(pool: PgPool) {
    set_operator(&pool).await;
    // Only an API key may be read-only (K2 0010-0029), and it is spelled `anima_` + token.
    let token = format!("anima_{}", URL_SAFE_NO_PAD.encode([5_u8; 32]));
    sqlx::query(
        "insert into access.api_tokens \
         (id, operator_id, token_hash, kind, made_for, name, display_prefix, scope, expires_at, created_at) \
         select gen_random_uuid(), id, $1, 'api_key', 'user', 'reader', 'anima_BQUFBQ', 'read', $2, $3 \
         from access.operators",
    )
    .bind(Sha256::digest([5_u8; 32]).to_vec())
    .bind(start() + chrono::TimeDelta::days(1))
    .bind(start())
    .execute(&pool)
    .await
    .unwrap();
    let hub = Hub::new(&pool, start());

    let me = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(me["data"]["me"]["username"], USERNAME, "{me}");
    // Signing out would delete the key: an API key may not (D-23, WP-43 A-6).
    let logout = hub
        .post(Some(&token), "mutation { logout }", json!({}))
        .await;
    assert_eq!(
        logout["errors"][0]["extensions"]["code"], "FORBIDDEN",
        "{logout}"
    );
    let again = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(again["data"]["me"]["username"], USERNAME, "{again}");
}
