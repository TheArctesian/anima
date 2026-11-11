//! The hub's routes with the agent routes inside the request log and behind the permitted
//! networks, the composition the programme's `main.rs` handoff is to mount, at a clock the test
//! moves; and the requests an operator and an agent send.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use anima_hub::{
    access,
    clock::Clock,
    events::Bus,
    logged,
    metrics::{Feed, agent_router},
    router,
    schema::build_schema,
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use http_body_util::BodyExt as _;
use ipnet::IpNet;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::hub::{dependencies, set_operator, start};

pub const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
pub const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
/// Addresses inside the permitted networks, and one outside them.
pub const PERMITTED: &str = "100.64.0.7";
pub const NEIGHBOUR: &str = "100.64.0.8";
pub const OUTSIDE: &str = "192.0.2.9";

/// A clock the test sets.
pub struct SetClock(Mutex<DateTime<Utc>>);

impl SetClock {
    pub fn advance(&self, by: TimeDelta) {
        *self.0.lock().unwrap() += by;
    }
}

impl Clock for SetClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

/// The hub's routes with the agent routes, composed as the `main.rs` handoff is to mount them.
pub fn app(pool: &PgPool, clock: Arc<dyn Clock>) -> Router {
    let bus = Bus::new(64);
    let schema = build_schema(dependencies(pool, clock.clone(), bus.clone()));
    let agent = agent_router(pool.clone(), Feed::new(), bus.clone(), clock.clone());
    let edge = access::Edge::new(pool.clone(), bus, clock.clone());
    router(schema, pool.clone(), clock)
        .merge(logged(agent))
        .layer(access::permitted_networks_layer(edge))
}

/// The hub at `start()` with the fixture operator signed in.
pub struct Agents {
    router: Router,
    pub clock: Arc<SetClock>,
    pub operator: String,
}

impl Agents {
    pub async fn new(pool: &PgPool) -> Self {
        Self::at(pool, start()).await
    }

    /// The hub with its clock at `now`. A push stores samples, whose partitions the database
    /// makes only near its own clock, so a push test starts at [`pushing_now`].
    pub async fn at(pool: &PgPool, now: DateTime<Utc>) -> Self {
        set_operator(pool).await;
        let networks: Vec<IpNet> = vec![
            "100.64.0.0/10".parse().unwrap(),
            "127.0.0.0/8".parse().unwrap(),
        ];
        access::permit_networks(pool, &networks).await.unwrap();
        let clock = Arc::new(SetClock(Mutex::new(now)));
        let mut agents = Self {
            router: app(pool, clock.clone()),
            clock,
            operator: String::new(),
        };
        agents.operator = agents.sign_in().await;
        agents
    }

    /// Sends `request` from `from`; the status and the body text.
    pub async fn send(&self, mut request: Request<Body>, from: &str) -> (StatusCode, String) {
        let peer: SocketAddr = format!("{from}:50000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    /// A GraphQL request with `token` as the bearer, from `PERMITTED`.
    pub async fn graphql(&self, token: Option<&str>, query: &str, variables: Value) -> Value {
        let body = json!({ "query": query, "variables": variables }).to_string();
        let mut request =
            Request::post("/graphql").header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let (status, text) = self
            .send(request.body(Body::from(body)).unwrap(), PERMITTED)
            .await;
        assert_eq!(status, StatusCode::OK, "{text}");
        serde_json::from_str(&text).unwrap()
    }

    /// A GraphQL request as the signed-in operator.
    pub async fn operate(&self, query: &str, variables: Value) -> Value {
        self.graphql(Some(&self.operator), query, variables).await
    }

    pub async fn sign_in(&self) -> String {
        let query = "mutation($input: LoginInput!) { login(input: $input) { token } }";
        let input = json!({ "input": { "username": crate::hub::USERNAME, "password": crate::hub::PASSWORD } });
        let response = self.graphql(None, query, input).await;
        response["data"]["login"]["token"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// The `enrolAgent` answer for `device`.
    pub async fn enrol_agent_response(&self, device: &str) -> Value {
        let query = "mutation($id: ID!) { enrolAgent(deviceId: $id) { code expiresAt } }";
        self.operate(query, json!({ "id": device })).await
    }

    /// A new enrolment code for `device`.
    pub async fn code_for(&self, device: &str) -> String {
        let response = self.enrol_agent_response(device).await;
        let code = response["data"]["enrolAgent"]["code"].as_str();
        code.unwrap_or_else(|| panic!("{response}")).to_owned()
    }

    /// `POST /agent/v1/enrol` with `body`, from `from`.
    pub async fn enrol_body(&self, body: impl Into<Body>, from: &str) -> (StatusCode, String) {
        let request = Request::post("/agent/v1/enrol")
            .header(header::CONTENT_TYPE, "application/json")
            .body(body.into())
            .unwrap();
        self.send(request, from).await
    }

    /// `POST /agent/v1/enrol` with `code`, from `PERMITTED`.
    pub async fn enrol(&self, code: &str) -> (StatusCode, String) {
        let body = json!({ "code": code }).to_string();
        self.enrol_body(body, PERMITTED).await
    }

    /// The `Enrolled` answer to `code`, which must be accepted.
    pub async fn enrolled(&self, code: &str) -> Value {
        let (status, text) = self.enrol(code).await;
        assert_eq!(status, StatusCode::OK, "{text}");
        serde_json::from_str(&text).unwrap()
    }

    /// A token of a fresh enrolment of `device`.
    pub async fn token_for(&self, device: &str) -> String {
        let code = self.code_for(device).await;
        self.enrolled(&code).await["token"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// `POST /agent/v1/renew` with `authorization` and `body`, from `from`.
    pub async fn renew_raw(
        &self,
        authorization: Option<&str>,
        body: impl Into<Body>,
        from: &str,
    ) -> (StatusCode, String) {
        let mut request =
            Request::post("/agent/v1/renew").header(header::CONTENT_TYPE, "application/json");
        if let Some(value) = authorization {
            request = request.header(header::AUTHORIZATION, value);
        }
        self.send(request.body(body.into()).unwrap(), from).await
    }

    /// `POST /agent/v1/renew` with `token` as the bearer and the body `{}`.
    pub async fn renew(&self, token: &str) -> (StatusCode, String) {
        self.renew_raw(Some(&format!("Bearer {token}")), "{}", PERMITTED)
            .await
    }

    /// The token of a renewal with `token`, which must be accepted.
    pub async fn renewed(&self, token: &str) -> String {
        let (status, text) = self.renew(token).await;
        assert_eq!(status, StatusCode::OK, "{text}");
        let renewed: Value = serde_json::from_str(&text).unwrap();
        renewed["token"].as_str().unwrap().to_owned()
    }
}

impl Agents {
    /// `POST /agent/v1/push` with `authorization` and `body`, from `from`.
    pub async fn push_raw(
        &self,
        authorization: Option<&str>,
        body: impl Into<Body>,
        from: &str,
    ) -> (StatusCode, String) {
        let mut request =
            Request::post("/agent/v1/push").header(header::CONTENT_TYPE, "application/json");
        if let Some(value) = authorization {
            request = request.header(header::AUTHORIZATION, value);
        }
        self.send(request.body(body.into()).unwrap(), from).await
    }

    /// `POST /agent/v1/push` of `batch` with `token` as the bearer, from `PERMITTED`.
    pub async fn push(&self, token: &str, batch: &Value) -> (StatusCode, String) {
        let bearer = format!("Bearer {token}");
        self.push_raw(Some(&bearer), batch.to_string(), PERMITTED)
            .await
    }
}

/// The database's clock cut down to a multiple of 10 seconds, where a pushed point may end.
pub async fn pushing_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(10)).unwrap()
}

/// A refusal body of K8 Section 6.
pub fn refusal(code: &str) -> String {
    format!(r#"{{"error":"{code}"}}"#)
}

/// The device's stored metrics source.
pub async fn source(pool: &PgPool, device: &str) -> String {
    sqlx::query_scalar("SELECT metrics_source FROM devices.devices WHERE id = $1::uuid")
        .bind(device)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The messages of the event log's metrics events with `severity`, oldest first.
pub async fn metrics_events(pool: &PgPool, severity: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT message FROM events.event_log WHERE source = 'metrics' AND severity = $1 \
         ORDER BY at, message",
    )
    .bind(severity)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// How many rows `table` of schema `metrics` holds for `device`.
pub async fn rows_of(pool: &PgPool, table: &str, device: &str) -> i64 {
    let sql = format!("SELECT count(*) FROM metrics.{table} WHERE device_id = $1::uuid");
    sqlx::query_scalar(&sql)
        .bind(device)
        .fetch_one(pool)
        .await
        .unwrap()
}
