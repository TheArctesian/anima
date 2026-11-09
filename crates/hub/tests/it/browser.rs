//! Browser sign-in through the hub's router at a public origin: `signInBrowser`, the
//! `__Host-anima` cookie, the Origin check on `POST /graphql` and the WebSocket upgrade, and
//! `logout` (D-3, D-4, SEC-002, API-003, WP-43 A-3 to A-8).

mod cookie;
mod sign_in;
pub(crate) mod socket;
mod table;

use std::{net::SocketAddr, sync::Arc};

use anima_hub::{
    access::PublicOrigin,
    clock::{Clock, FixedClock},
    events::Bus,
    router_at,
    schema::build_schema,
};
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use http_body_util::BodyExt as _;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::hub::{CLIENT, PASSWORD, USERNAME, dependencies};

/// The public origin the test hub serves.
const PUBLIC: &str = "https://hub.anima.ing";

const SIGN_IN: &str = "mutation($input: LoginInput!) { \
    signInBrowser(input: $input) { expiresAt operator { username totpEnabled } } }";

/// The router as `main.rs` builds it with `ANIMA_PUBLIC_ORIGIN`, the clock stopped at `now`.
fn browser_hub(pool: &PgPool, now: DateTime<Utc>) -> Router {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let schema = build_schema(dependencies(pool, clock.clone(), Bus::new(16)));
    router_at(
        schema,
        pool.clone(),
        clock,
        PublicOrigin::new(PUBLIC.to_owned()),
    )
}

/// What the hub answered.
struct Answer {
    headers: HeaderMap,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap()
    }

    /// Every `Set-Cookie` value, as text.
    fn set_cookies(&self) -> Vec<String> {
        let values = self.headers.get_all(header::SET_COOKIE).iter();
        values
            .map(|value| value.to_str().unwrap().to_owned())
            .collect()
    }

    /// The browser token the one `Set-Cookie` gives.
    fn token(&self) -> String {
        let [cookie] = self.set_cookies().try_into().expect("one Set-Cookie");
        let value = cookie.strip_prefix("__Host-anima=").expect("the cookie");
        value.split(';').next().unwrap().to_owned()
    }
}

/// Posts `query` with `variables` and the extra `headers` to `/graphql` from `CLIENT`.
async fn post(router: &Router, headers: &[(&str, &str)], query: &str, variables: Value) -> Answer {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let mut request = Request::post("/graphql").header(header::CONTENT_TYPE, "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut request = request.body(Body::from(body)).unwrap();
    let peer: SocketAddr = format!("{CLIENT}:50000").parse().unwrap();
    request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(peer));
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    Answer { headers, body }
}

/// Runs `signInBrowser` with `password` and the extra `headers`.
async fn sign_in_with(router: &Router, headers: &[(&str, &str)], password: &str) -> Answer {
    let input = json!({ "input": { "username": USERNAME, "password": password } });
    post(router, headers, SIGN_IN, input).await
}

/// Signs a browser in from the public origin; returns its token.
async fn browser_token(router: &Router) -> String {
    let answer = sign_in_with(router, &[("origin", PUBLIC)], PASSWORD).await;
    assert!(
        answer.json()["data"]["signInBrowser"].is_object(),
        "{}",
        answer.body
    );
    answer.token()
}

/// The cookie header that carries `token`.
fn cookie(token: &str) -> String {
    format!("__Host-anima={token}")
}

/// The username `me` answers with these headers, or the error code.
async fn me(router: &Router, headers: &[(&str, &str)]) -> Result<String, String> {
    let answer = post(router, headers, "{ me { username } }", json!({}))
        .await
        .json();
    match answer["data"]["me"]["username"].as_str() {
        Some(name) => Ok(name.to_owned()),
        None => Err(answer["errors"][0]["extensions"]["code"]
            .as_str()
            .unwrap()
            .to_owned()),
    }
}

/// The stored hash of a token's text.
fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(URL_SAFE_NO_PAD.decode(token).unwrap()).to_vec()
}
