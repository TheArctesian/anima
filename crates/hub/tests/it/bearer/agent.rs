//! R-092 D-1: an agent token or an enrolment code is signed out on `/graphql`, the socket, the
//! tunnel and `/metrics`, while an API key whose text begins `agent_` or `enrol_` still works
//! (MET-012, MET-013, K8).

use anima_hub::access;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    LOGIN, PERMITTED, READ_KEY, app, assert_refused, database_now, scrape_with, send, stored,
    stored_bytes,
};

/// `{ me { username } }` on `/graphql` with `Authorization: Bearer <token>`.
async fn me(app: &axum::Router, token: &str) -> Value {
    let request = Request::post("/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(r#"{"query":"{ me { username } }"}"#))
        .unwrap();
    serde_json::from_str(&send(app, PERMITTED, request).await.body).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn r_092_met_013_agent_secrets_are_signed_out_everywhere(pool: PgPool) {
    let now = database_now(&pool).await;
    let login = stored(&pool, LOGIN, now + TimeDelta::days(1)).await;
    let (app, _web) = app(&pool, now).await;
    // The control: the stored login token signs in.
    assert_eq!(me(&app, &login).await["data"]["me"]["username"], "operator");

    // An agent token whose 43 characters are a stored login token's text: refused by its
    // spelling, and by the token length rule too (55 characters are neither token nor key).
    let agent = format!("anima_agent_{login}");
    let enrolment = format!("anima_enrol_{}", URL_SAFE_NO_PAD.encode([3_u8; 16]));
    for secret in [agent, enrolment] {
        let signed_out = me(&app, &secret).await;
        assert_eq!(
            signed_out["errors"][0]["extensions"]["code"],
            "UNAUTHENTICATED"
        );
        assert_eq!(signed_out["errors"][0]["message"], "Sign in to continue.");
        assert_refused(&scrape_with(&app, &secret).await, &secret[..12]);
        let payload = json!({ "authorization": format!("Bearer {secret}") });
        let socket = access::identify_connection(&pool, &payload, now)
            .await
            .unwrap();
        assert!(socket.is_none());
        let tunnel = access::operator_for_token(&pool, &secret, now).await;
        assert!(matches!(tunnel, Err(access::Error::Unauthenticated)));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_092_an_api_key_whose_text_begins_agent_or_enrol_is_still_a_key(pool: PgPool) {
    let now = database_now(&pool).await;
    let (app, _web) = app(&pool, now).await;
    for (fill, begins) in [(11, "agent_"), (12, "enrol_")] {
        let text = format!("{begins}{}", "A".repeat(37));
        let bytes: [u8; 32] = URL_SAFE_NO_PAD.decode(&text).unwrap().try_into().unwrap();
        let mut key = READ_KEY;
        key.fill = fill;
        let presented = stored_bytes(&pool, key, bytes, now + TimeDelta::days(1)).await;
        assert!(presented.starts_with(&format!("anima_{begins}")));

        let scraped = scrape_with(&app, &presented).await;
        assert_eq!(scraped.status, StatusCode::OK, "{scraped:?}");
        assert_eq!(
            me(&app, &presented).await["data"]["me"]["username"],
            "operator"
        );
    }
}
