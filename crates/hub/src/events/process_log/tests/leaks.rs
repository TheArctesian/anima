//! Secrets in GraphQL errors never reach the process log: argument values async-graphql echoes,
//! and credentials in a resolver's message (LOG-007, API-010, K7).

use async_graphql::{
    EmptyMutation, EmptySubscription, InputObject, InputValueError, InputValueResult, Object,
    OneofObject, Request, Response, Scalar, ScalarType, Schema, Value, Variables,
};
use serde_json::json;

use super::capture::capture_log;
use crate::{events::process_log::internal_errors::is_argument_error, schema::harden};

const SECRET: &str = "hunter2-correct-horse";

#[derive(InputObject)]
struct SignIn {
    username: String,
    password: String,
}

#[derive(OneofObject)]
enum Proof {
    Password(String),
    Code(String),
}

/// A scalar that refuses every value, quoting it as many parsers do.
struct Pin;

#[Scalar]
impl ScalarType for Pin {
    fn parse(value: Value) -> InputValueResult<Self> {
        Err(InputValueError::custom(format!("{value} is not a PIN")))
    }

    fn to_value(&self) -> Value {
        Value::Null
    }
}

struct Query;

#[Object]
impl Query {
    async fn login(&self, input: SignIn) -> bool {
        input.username.is_empty() && input.password.is_empty()
    }

    async fn prove(&self, proof: Proof) -> bool {
        matches!(proof, Proof::Code(_))
    }

    async fn unlock(&self, pin: Pin) -> bool {
        let _ = pin;
        true
    }

    async fn connect(&self) -> async_graphql::Result<bool> {
        Err(async_graphql::Error::new(format!(
            "cannot reach postgres://hub:{SECRET}@db.lan/anima"
        )))
    }
}

/// Requests whose argument async-graphql cannot coerce while resolving, each holding `SECRET`.
fn malformed() -> [Request; 3] {
    let variables = |input| Variables::from_json(json!({ "input": input }));
    [
        Request::new("query($input: SignIn!) { login(input: $input) }")
            .variables(variables(json!(SECRET))),
        Request::new("query($input: Proof!) { prove(proof: $input) }")
            .variables(variables(json!(SECRET))),
        Request::new(format!(r#"{{ unlock(pin: "{SECRET}") }}"#)),
    ]
}

fn plain() -> Schema<Query, EmptyMutation, EmptySubscription> {
    Schema::new(Query, EmptyMutation, EmptySubscription)
}

fn hardened() -> Schema<Query, EmptyMutation, EmptySubscription> {
    harden(Schema::build(Query, EmptyMutation, EmptySubscription), true).finish()
}

/// Pins async-graphql's own shape: an upgrade that changes it fails here, not in production.
#[tokio::test]
async fn log_007_async_graphql_echoes_a_bad_argument_with_a_path_and_is_recognised() {
    for request in malformed() {
        let response: Response = plain().execute(request).await;
        assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
        let error = &response.errors[0];
        assert!(!error.path.is_empty(), "{error:?}");
        assert!(error.message.contains(SECRET), "{error:?}");
        assert!(is_argument_error(&error.message), "{error:?}");
    }
}

#[tokio::test]
async fn log_007_a_bad_argument_is_validation_and_its_value_stays_out_of_the_log() {
    let (capture, _guard) = capture_log("trace");

    for request in malformed() {
        let response = hardened().execute(request).await;
        let error = serde_json::to_value(&response.errors[0]).unwrap();
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
        assert!(
            error["extensions"]["fields"][0]["field"].is_string(),
            "{error}"
        );
        assert!(!error.to_string().contains(SECRET), "{error}");
    }

    let text = capture.text();
    assert_eq!(text.matches("argument refused").count(), 3, "{text}");
    assert!(!text.contains(SECRET), "{text}");
}

#[test]
fn log_007_resolver_text_is_not_an_argument_error() {
    assert!(!is_argument_error(
        "relation \"secret_table\" does not exist"
    ));
    assert!(!is_argument_error("stream broke: Failed to parse \"X\""));
}

#[tokio::test]
async fn log_007_a_resolver_message_is_logged_with_its_credentials_masked() {
    let (capture, _guard) = capture_log("trace");

    let response = hardened().execute("{ connect }").await;

    assert_eq!(
        response.errors[0].message,
        "Something went wrong on the hub."
    );
    let text = capture.text();
    assert!(text.contains("cannot reach postgres://hub:"), "{text}");
    assert!(text.contains("@db.lan/anima"), "{text}");
    assert!(!text.contains(SECRET), "{text}");
}
