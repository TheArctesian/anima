#![expect(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    reason = "async-graphql resolvers must be async"
)]

use async_graphql::{
    EmptyMutation, EmptySubscription, Error as GraphqlError, ErrorExtensions, Object, Request,
    Response, Schema, Subscription, Variables,
    futures_util::{Stream, StreamExt, stream},
    value,
};

mod capture;

use self::capture::{Capture, capture_log};
use super::{Error, subscriber};
use crate::schema::harden;

struct Node;

#[Object]
impl Node {
    async fn id(&self) -> i32 {
        1
    }

    async fn child(&self) -> Node {
        Node
    }

    async fn leak(&self) -> async_graphql::Result<String> {
        Err(GraphqlError::new("connection to /var/run/pg.sock refused"))
    }
}

struct Feed;

#[Subscription]
impl Feed {
    async fn ticks(&self) -> impl Stream<Item = async_graphql::Result<i32>> {
        stream::iter([Ok(1), Err(GraphqlError::new("stream broke at /srv/db"))])
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
    let (capture, _guard) = capture_log("info");

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
    let (capture, _guard) = capture_log("info");

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
    let (capture, _guard) = capture_log("trace");

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

#[tokio::test]
async fn api_010_error_in_a_nested_resolver_is_masked() {
    let response = schema(true).execute("{ node { child { leak } } }").await;
    let error = first_error(&response);
    assert_eq!(error["message"], "Something went wrong on the hub.");
    assert_eq!(error["extensions"]["code"], "INTERNAL");
}

#[tokio::test]
async fn api_010_and_log_002_subscription_items_are_masked_and_carry_a_request_id() {
    let (capture, _guard) = capture_log("info");
    let schema = harden(Schema::build(Query, EmptyMutation, Feed), true).finish();

    let items: Vec<Response> = schema
        .execute_stream("subscription { ticks }")
        .collect()
        .await;

    assert_eq!(items.len(), 2);
    assert!(items[0].errors.is_empty(), "{:?}", items[0].errors);
    let error = first_error(&items[1]);
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
            .contains("/srv/db")
    );
    assert!(entry["span"]["request_id"].is_string());
}

#[tokio::test]
async fn log_002_concurrent_requests_each_log_their_own_request_id() {
    let (capture, _guard) = capture_log("info");
    let schema = schema(true);

    let (first, second) = tokio::join!(
        schema.execute("{ login(password: \"a\") }"),
        schema.execute("{ login(password: \"b\") }"),
    );

    let header = |response: &Response| {
        response
            .http_headers
            .get("x-request-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    };
    let logged: Vec<String> = capture
        .lines()
        .iter()
        .map(|entry| entry["span"]["request_id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(logged.len(), 2);
    assert_ne!(header(&first), header(&second));
    assert!(logged.contains(&header(&first)));
    assert!(logged.contains(&header(&second)));
}
