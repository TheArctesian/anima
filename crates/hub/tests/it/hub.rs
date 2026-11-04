//! A hub router on a test database at a fixed time, and requests to its `/graphql` endpoint.

use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use anima_hub::{
    access::{self, Password},
    clock::{Clock, FixedClock},
    config,
    events::Bus,
    router,
    schema::build_schema,
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, header},
};
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

pub const USERNAME: &str = "operator";
pub const PASSWORD: &str = "correct horse battery staple";
pub const CLIENT: &str = "100.64.0.7";

pub fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

/// The hub's router as `main.rs` builds it, with the clock stopped at `now`.
pub struct Hub {
    router: Router,
    pub bus: Bus,
}

impl Hub {
    pub fn new(pool: &PgPool, now: DateTime<Utc>) -> Self {
        let config = Arc::new(config::from_env(&BTreeMap::new()).unwrap());
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
        let bus = Bus::new(16);
        let schema = build_schema(config, clock.clone(), pool.clone(), bus.clone());
        Self {
            router: router(schema, pool.clone(), clock),
            bus,
        }
    }

    /// Posts a GraphQL request from `CLIENT`; returns the response body as JSON text.
    pub async fn post_text(&self, token: Option<&str>, query: &str, variables: Value) -> String {
        let body = json!({ "query": query, "variables": variables }).to_string();
        let mut request = Request::post("/graphql").header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let mut request = request.body(Body::from(body)).unwrap();
        let peer: SocketAddr = format!("{CLIENT}:50000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = self.router.clone().oneshot(request).await.unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    pub async fn post(&self, token: Option<&str>, query: &str, variables: Value) -> Value {
        serde_json::from_str(&self.post_text(token, query, variables).await).unwrap()
    }

    /// Runs the `login` mutation with `username` and `password`.
    pub async fn login(&self, username: &str, password: &str) -> Value {
        let query = "mutation($input: LoginInput!) { \
            login(input: $input) { token expiresAt operator { id username totpEnabled } } }";
        let input = json!({ "input": { "username": username, "password": password } });
        self.post(None, query, input).await
    }

    /// Signs in as the operator and returns the token.
    pub async fn token(&self) -> String {
        let response = self.login(USERNAME, PASSWORD).await;
        response["data"]["login"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("login failed: {response}"))
            .to_owned()
    }
}

/// Creates the fixture operator (K6 `phase1.json`).
pub async fn set_operator(pool: &PgPool) {
    access::set_operator(pool, USERNAME, Password::new(PASSWORD.to_owned()), start())
        .await
        .unwrap();
}

/// The one error of a response, as `(message, code)`.
pub fn error(response: &Value) -> (String, String) {
    let errors = response["errors"].as_array().unwrap_or_else(|| panic!("no errors: {response}"));
    assert_eq!(errors.len(), 1, "{response}");
    let message = errors[0]["message"].as_str().unwrap().to_owned();
    let code = errors[0]["extensions"]["code"].as_str().unwrap().to_owned();
    (message, code)
}

pub fn unauthenticated() -> (String, String) {
    ("Sign in to continue.".to_owned(), "UNAUTHENTICATED".to_owned())
}

pub fn wrong_credentials() -> (String, String) {
    (
        "The username or password is wrong.".to_owned(),
        "UNAUTHENTICATED".to_owned(),
    )
}
