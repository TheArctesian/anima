#![expect(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    reason = "async-graphql resolvers must be async"
)]

use std::{
    io,
    sync::{Arc, Mutex},
};

use async_graphql::{
    EmptyMutation, EmptySubscription, Error as GraphqlError, ErrorExtensions, Object, Request,
    Response, Schema, Variables, value,
};
use serde_json::Value;

use super::{Error, subscriber};
use crate::schema::harden;

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

impl Capture {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }

    fn lines(&self) -> Vec<Value> {
        self.text()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

struct Node;

#[Object]
impl Node {
    async fn id(&self) -> i32 {
        1
    }

    async fn child(&self) -> Node {
        Node
    }
}

struct Query;

#[Object]
impl Query {
    async fn node(&self) -> Node {
        Node
    }

    async fn id(&self) -> i32 {
        1
    }

    async fn broken(&self) -> async_graphql::Result<String> {
        Err(GraphqlError::new(
            "relation \"secret_table\" does not exist",
        ))
    }

    async fn missing(&self) -> async_graphql::Result<String> {
        Err(GraphqlError::new("No such device.").extend_with(|_, e| e.set("code", "NOT_FOUND")))
    }

    async fn login(&self, password: String) -> async_graphql::Result<bool> {
        tracing::info!("login attempted");
        Ok(password.is_empty())
    }
}

type TestSchema = Schema<Query, EmptyMutation, EmptySubscription>;

fn schema(production: bool) -> TestSchema {
    harden(
        Schema::build(Query, EmptyMutation, EmptySubscription),
        production,
    )
    .finish()
}

/// A query selecting `depth` nested fields: `{ node { child { ... id } } }`.
fn nested(depth: usize) -> String {
    let mut query = "id".to_owned();
    for _ in 1..depth {
        query = format!("child {{ {query} }}");
    }
    format!("{{ node {{ {query} }} }}")
}

fn flat(fields: usize) -> String {
    let selections: Vec<String> = (0..fields).map(|n| format!("a{n}: id")).collect();
    format!("{{ {} }}", selections.join(" "))
}

fn first_error(response: &Response) -> serde_json::Value {
    serde_json::to_value(&response.errors[0]).unwrap()
}

#[tokio::test]
async fn api_004_query_deeper_than_ten_is_refused() {
    // `node` is depth 1, so `nested(9)` reaches depth 10 and `nested(10)` depth 11.
    let at_limit = schema(false).execute(nested(9)).await;
    assert!(at_limit.errors.is_empty(), "{:?}", at_limit.errors);
    let too_deep = schema(false).execute(nested(10)).await;
    assert_eq!(too_deep.errors.len(), 1);
    assert!(too_deep.errors[0].message.contains("nested too deep"));
}

#[tokio::test]
async fn api_004_query_above_the_complexity_limit_is_refused() {
    let at_limit = schema(false).execute(flat(500)).await;
    assert!(at_limit.errors.is_empty(), "{:?}", at_limit.errors);
    let too_big = schema(false).execute(flat(501)).await;
    assert_eq!(too_big.errors.len(), 1);
    assert!(too_big.errors[0].message.contains("too complex"));
}

#[tokio::test]
async fn api_007_introspection_is_refused_in_production_and_allowed_otherwise() {
    let query = "{ __schema { queryType { name } } }";
    let development = schema(false).execute(query).await;
    assert!(development.errors.is_empty(), "{:?}", development.errors);
    assert_eq!(
        development.data,
        value!({ "__schema": { "queryType": { "name": "Query" } } })
    );
    let production = schema(true).execute(query).await;
    // async-graphql answers a disabled introspection with null, not an error.
    assert_eq!(production.data, value!({ "__schema": null }));
}

#[tokio::test]
async fn api_010_internal_error_shows_the_generic_message_and_the_log_keeps_the_original() {
    let capture = Capture::default();
    let log = subscriber("info", {
        let capture = capture.clone();
        move || capture.clone()
    })
    .unwrap();
    let _guard = tracing::subscriber::set_default(log);

    let response = schema(true).execute("{ broken }").await;

    let error = first_error(&response);
    assert_eq!(error["message"], "Something went wrong on the hub.");
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    let entries = capture.lines();
    let entry = entries
        .iter()
        .find(|entry| entry["level"] == "ERROR")
        .expect("an error line");
    assert!(
        entry["fields"]["message"]
            .as_str()
            .unwrap()
            .contains("relation \"secret_table\" does not exist")
    );
    assert!(entry["span"]["request_id"].is_string());
}

#[tokio::test]
async fn api_010_error_with_a_code_passes_unchanged() {
    let response = schema(true).execute("{ missing }").await;
    let error = first_error(&response);
    assert_eq!(error["message"], "No such device.");
    assert_eq!(error["extensions"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn log_002_line_has_time_level_target_message_and_request_id() {
    let capture = Capture::default();
    let log = subscriber("info", {
        let capture = capture.clone();
        move || capture.clone()
    })
    .unwrap();
    let _guard = tracing::subscriber::set_default(log);

    tracing::info!("outside any request");
    let response = schema(true)
        .execute(Request::new("{ login(password: \"\") }"))
        .await;

    let entries = capture.lines();
    let outside = &entries[0];
    for key in ["timestamp", "level", "target"] {
        assert!(outside[key].is_string(), "{key} in {outside}");
    }
    assert_eq!(outside["fields"]["message"], "outside any request");
    assert!(outside["span"].is_null());
    let inside = &entries[1];
    assert_eq!(inside["fields"]["message"], "login attempted");
    let request_id = inside["span"]["request_id"].as_str().unwrap();
    assert_eq!(
        uuid::Uuid::parse_str(request_id).unwrap().get_version_num(),
        7
    );
    assert_eq!(
        response.http_headers.get("x-request-id").unwrap(),
        request_id
    );
}

#[tokio::test]
async fn log_007_password_variable_never_reaches_the_log() {
    let capture = Capture::default();
    let log = subscriber("trace", {
        let capture = capture.clone();
        move || capture.clone()
    })
    .unwrap();
    let _guard = tracing::subscriber::set_default(log);

    let request = Request::new("query($p: String!) { login(password: $p) }").variables(
        Variables::from_json(serde_json::json!({ "p": "hunter2-correct-horse" })),
    );
    schema(true).execute(request).await;

    let text = capture.text();
    assert!(text.contains("login attempted"));
    assert!(!text.contains("hunter2-correct-horse"));
}

#[test]
fn log_003_filter_limits_levels_and_an_invalid_filter_is_a_typed_error() {
    let capture = Capture::default();
    let log = subscriber("warn", {
        let capture = capture.clone();
        move || capture.clone()
    })
    .unwrap();
    tracing::subscriber::with_default(log, || {
        tracing::info!("hidden");
        tracing::warn!("shown");
    });
    let entries = capture.lines();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["fields"]["message"], "shown");

    let result = subscriber("info,[bad", Capture::default);
    assert!(matches!(result, Err(Error::Filter { .. })));
}
