//! The checks schema as `schema.rs` will build it, and requests from a signed-in operator.

use std::{
    future::Future,
    io::{self, Write as _},
    sync::{
        Arc, Mutex,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};

use anima_hub::{
    access::Identity,
    checks::{
        self, Beat, CheckState,
        graphql::{
            Mutation, Query, Subscription, check_status_loader, check_uptime_loader,
            heartbeats_loader, last_heartbeat_loader,
        },
    },
    clock::{Clock, FixedClock},
    credentials::Vault,
    events::{Bus, ClientAddr, Operator, process_log::subscriber},
    ids::{CheckId, OperatorId},
    schema::harden,
    status::Status,
};
use async_graphql::{Request, Response, Schema, Variables, dataloader::DataLoader};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

pub type TestSchema = Schema<Query, Mutation, Subscription>;

pub const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
pub const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
pub const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
pub const PVE_01_PING: &str = "0190a000-0000-7000-8003-000000000001";

/// The instant the test clock reports.
pub fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

pub fn vault() -> Vault {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

pub fn schema_at(pool: &PgPool, at: DateTime<Utc>) -> (TestSchema, Bus) {
    schema_with_window(pool, at, None)
}

/// How long the batching tests let a loader collect keys before it runs its batch.
///
/// Under machine load a short window can close before every field of one query has asked, so
/// one query becomes several batches and the statement count grows. Half a second is far longer
/// than async-graphql's default window, and a batch still runs as soon as the window ends, so a
/// test pays at most this once per loader round.
const BATCH_WINDOW: Duration = Duration::from_millis(500);

/// The schema of `schema_at` whose loaders wait `BATCH_WINDOW`, for tests that count statements.
pub fn schema_batched_at(pool: &PgPool, at: DateTime<Utc>) -> TestSchema {
    schema_with_window(pool, at, Some(BATCH_WINDOW)).0
}

fn wait<T>(loader: DataLoader<T>, window: Option<Duration>) -> DataLoader<T> {
    match window {
        Some(window) => loader.delay(window),
        None => loader,
    }
}

fn schema_with_window(
    pool: &PgPool,
    at: DateTime<Utc>,
    window: Option<Duration>,
) -> (TestSchema, Bus) {
    let bus = Bus::new(16);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(at));
    let schema = Schema::build(Query, Mutation, Subscription)
        .data(pool.clone())
        .data(vault())
        .data(bus.clone())
        .data(wait(
            check_status_loader(pool.clone(), clock.clone()),
            window,
        ))
        .data(wait(last_heartbeat_loader(pool.clone()), window))
        .data(wait(heartbeats_loader(pool.clone()), window))
        .data(wait(
            check_uptime_loader(pool.clone(), clock.clone()),
            window,
        ))
        .data(clock)
        .finish();
    (schema, bus)
}

pub fn schema_with_bus(pool: &PgPool) -> (TestSchema, Bus) {
    schema_at(pool, now())
}

pub fn schema_for(pool: &PgPool) -> TestSchema {
    schema_with_bus(pool).0
}

/// The schema with the process-log extensions `schema.rs` adds.
pub fn hardened_schema(pool: &PgPool) -> TestSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let builder = Schema::build(Query, Mutation, Subscription)
        .data(pool.clone())
        .data(vault())
        .data(Bus::new(16))
        .data(check_status_loader(pool.clone(), clock.clone()))
        .data(last_heartbeat_loader(pool.clone()))
        .data(heartbeats_loader(pool.clone()))
        .data(check_uptime_loader(pool.clone(), clock.clone()))
        .data(clock);
    harden(builder, false).finish()
}

/// A request from the K6 operator.
pub fn signed_in(query: &str) -> Request {
    let operator = Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    };
    Identity::for_tests(operator, now() + TimeDelta::days(3650))
        .attach(Request::new(query))
        .data(ClientAddr("10.10.0.77".parse().unwrap()))
}

pub async fn data_with(schema: &TestSchema, query: &str, variables: Value) -> Value {
    let response = with_variables(schema, query, variables).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    response.data.into_json().unwrap()
}

pub async fn with_variables(schema: &TestSchema, query: &str, variables: Value) -> Response {
    let variables = Variables::from_json(variables);
    schema.execute(signed_in(query).variables(variables)).await
}

pub fn error_json(response: &Response) -> Value {
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    serde_json::to_value(&response.errors[0]).unwrap()
}

pub fn code(error: &Value) -> &str {
    error["extensions"]["code"].as_str().unwrap()
}

/// The field names of a `VALIDATION` error, in order.
pub fn validation_fields(error: &Value) -> Vec<&str> {
    assert_eq!(code(error), "VALIDATION", "{error}");
    error["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["field"].as_str().unwrap())
        .collect()
}

pub const CREATE: &str = "mutation($input: CreateCheckInput!) { createCheck(input: $input) \
    { id name type paused config { __typename } } }";
pub const UPDATE: &str = "mutation($id: ID!, $input: UpdateCheckInput!) { \
    updateCheck(id: $id, input: $input) { id name config { __typename } } }";

/// A `createCheck` input on `device` with this config.
pub fn input(device: &str, name: &str, config: Value) -> Value {
    let mut input = json!({ "deviceId": device, "name": name });
    input["config"] = config;
    input
}

/// Creates a check and returns its id.
pub async fn create(schema: &TestSchema, input: Value) -> String {
    let created = data_with(schema, CREATE, json!({ "input": input })).await;
    created["createCheck"]["id"].as_str().unwrap().to_owned()
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Runs the request with this thread's process log routed to a buffer; returns the log text
/// and the result. A control line proves the buffer receives the log.
pub async fn captured<F, R>(request: impl FnOnce() -> F) -> (String, R)
where
    F: Future<Output = R>,
{
    let capture = Capture::default();
    let writer = capture.clone();
    let log = subscriber("trace", move || writer.clone()).unwrap();
    let guard = tracing::subscriber::set_default(log);
    tracing::callsite::rebuild_interest_cache();
    let result = request().await;
    tracing::error!("capture control line");
    drop(guard);
    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("capture control line"),
        "the log was not captured"
    );
    (text, result)
}

/// Fails if `secret` appears in any of `places`, without printing the place.
pub fn assert_absent(secret: &str, places: &[&str]) {
    for (index, place) in places.iter().enumerate() {
        assert!(!place.contains(secret), "a secret appears in place {index}");
    }
}

/// A clock that reports one second later on every call, so two pushes never share an instant.
pub struct StepClock(AtomicI64, DateTime<Utc>);

impl StepClock {
    pub fn from(start: DateTime<Utc>) -> Self {
        Self(AtomicI64::new(0), start)
    }
}

impl Clock for StepClock {
    fn now(&self) -> DateTime<Utc> {
        self.1 + TimeDelta::seconds(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

/// The database clock: the partition functions accept only instants near it.
pub async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

pub fn beat(at: DateTime<Utc>, status: Status, response_time_ms: Option<u32>) -> Beat {
    Beat {
        at,
        status,
        response_time_ms,
        message: None,
        state: CheckState {
            status,
            failures: 0,
            next_due_at: None,
        },
        resolved_ip: None,
        cert_not_after: None,
    }
}

/// Records the beat on the check, as the scheduler does.
pub async fn record(pool: &PgPool, check: &str, beat: &Beat) {
    let mut conn = pool.acquire().await.unwrap();
    let recorded = checks::record_beat(&mut conn, check.parse::<CheckId>().unwrap(), beat)
        .await
        .unwrap();
    assert_eq!(recorded, checks::Recorded::Written);
}
