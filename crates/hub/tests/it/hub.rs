//! A hub router on a test database at a fixed time, and requests to its `/graphql` endpoint.

use std::{collections::BTreeMap, io::Write as _, net::SocketAddr, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD};

use anima_hub::{
    clock::{Clock, FixedClock},
    config,
    fleet::{
        access::{self, Password},
        credentials::Vault,
        events::Bus,
        metrics::Feed,
        sessions::Live,
        targets::AddressPolicy,
    },
    ids::DeviceId,
    router,
    schema::{Dependencies, build_schema},
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

pub const USERNAME: &str = "operator";
pub const PASSWORD: &str = "correct horse battery staple";
pub const CLIENT: &str = "100.64.0.7";

/// The device's host text now: what a credential stored for it is bound to (SEC-011).
pub async fn host_of(pool: &PgPool, device: DeviceId) -> String {
    sqlx::query_scalar("select host from devices.devices where id = $1")
        .bind(device)
        .fetch_one(pool)
        .await
        .unwrap()
}

pub fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

/// The hub's router as `main.rs` builds it, with the clock stopped at `now`.
pub struct Hub {
    router: Router,
    pub bus: Bus,
    client: &'static str,
}

impl Hub {
    pub fn new(pool: &PgPool, now: DateTime<Utc>) -> Self {
        Self::from_client(pool, now, CLIENT)
    }

    /// A hub whose requests come from `client` instead of `CLIENT`.
    pub fn from_client(pool: &PgPool, now: DateTime<Utc>, client: &'static str) -> Self {
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
        let bus = Bus::new(16);
        let schema = build_schema(dependencies(pool, clock.clone(), bus.clone()));
        Self {
            router: router(schema, pool.clone(), clock),
            bus,
            client,
        }
    }

    /// A hub over `dependencies`, at their clock, for a test that needs other parts than
    /// `dependencies` gives.
    pub fn serving(pool: &PgPool, dependencies: Dependencies) -> Self {
        let (clock, bus) = (dependencies.clock.clone(), dependencies.bus.clone());
        Self {
            router: router(build_schema(dependencies), pool.clone(), clock),
            bus,
            client: CLIENT,
        }
    }

    /// Sends a raw request from this hub's client; returns the status and the body text.
    pub async fn send(&self, request: Request<Body>) -> (StatusCode, String) {
        let mut request = request;
        let peer: SocketAddr = format!("{}:50000", self.client).parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Posts a GraphQL request; returns the response body as JSON text.
    pub async fn post_text(&self, token: Option<&str>, query: &str, variables: Value) -> String {
        let body = json!({ "query": query, "variables": variables }).to_string();
        let mut request =
            Request::post("/graphql").header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let (status, text) = self.send(request.body(Body::from(body)).unwrap()).await;
        assert!(status.is_success(), "{status} {text}");
        text
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

/// A vault over a throwaway key in a temporary file (SEC-004); never a real key file.
pub fn test_vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

/// The schema dependencies `main.rs` builds, with default config and a throwaway vault.
pub fn dependencies(pool: &PgPool, clock: Arc<dyn Clock>, bus: Bus) -> Dependencies {
    Dependencies {
        config: Arc::new(config::from_env(&BTreeMap::new()).unwrap()),
        clock,
        pool: pool.clone(),
        bus,
        vault: test_vault(),
        live: Live::default(),
        policy: AddressPolicy::default(),
        metrics_feed: Feed::new(),
        build_updates: anima_hub::fleet::builds::Updates::new(),
        bucket: None,
        vpn_feed: anima_hub::fleet::vpn::Feed::new(),
        vpn_broker: None,
    }
}

/// The SDL of the schema `build_schema` makes, which K1 describes (CON-002). The SDL needs no
/// database, so the pool never connects.
pub fn hub_sdl() -> String {
    let pool = PgPool::connect_lazy("postgres://unused@127.0.0.1:1/none").unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    build_schema(dependencies(&pool, clock, Bus::new(1))).sdl()
}

/// Creates the fixture operator (K6 `phase1.json`).
pub async fn set_operator(pool: &PgPool) {
    access::set_operator(pool, USERNAME, Password::new(PASSWORD.to_owned()), start())
        .await
        .unwrap();
}

/// The one error of a response, as `(message, code)`.
pub fn error(response: &Value) -> (String, String) {
    let errors = response["errors"]
        .as_array()
        .unwrap_or_else(|| panic!("no errors: {response}"));
    assert_eq!(errors.len(), 1, "{response}");
    let message = errors[0]["message"].as_str().unwrap().to_owned();
    let code = errors[0]["extensions"]["code"].as_str().unwrap().to_owned();
    (message, code)
}

pub fn unauthenticated() -> (String, String) {
    (
        "Sign in to continue.".to_owned(),
        "UNAUTHENTICATED".to_owned(),
    )
}

pub fn wrong_credentials() -> (String, String) {
    (
        "The username or password is wrong.".to_owned(),
        "UNAUTHENTICATED".to_owned(),
    )
}
