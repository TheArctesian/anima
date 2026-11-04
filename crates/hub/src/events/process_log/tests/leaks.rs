//! Secrets in GraphQL errors never reach the response or the process log: argument values
//! async-graphql echoes, and credentials in a resolver's message (LOG-007, API-010, K7).

use async_graphql::{
    EmptyMutation, EmptySubscription, InputObject, InputValueError, InputValueResult, Object,
    OneofObject, Pos, Request, Response, Scalar, ScalarType, Schema, Value, Variables,
};
use serde_json::json;

use super::capture::capture_log;
use crate::schema::harden;

pub(super) const SECRET: &str = "hunter2-correct-horse";

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

/// A scalar whose parser sets its own code and still quotes the value (COULD 2).
struct CodedPin;

#[Scalar]
impl ScalarType for CodedPin {
    fn parse(value: Value) -> InputValueResult<Self> {
        Err(InputValueError::custom(format!("{value} is not a PIN"))
            .with_extension("code", "VALIDATION")
            .with_extension("fields", Value::List(vec![value])))
    }

    fn to_value(&self) -> Value {
        Value::Null
    }
}

pub(super) struct Query;

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

    async fn coded(&self, pin: CodedPin) -> bool {
        let _ = pin;
        true
    }

    async fn connect(&self) -> async_graphql::Result<bool> {
        Err(async_graphql::Error::new(format!(
            "cannot reach postgres://hub:{SECRET}@db.lan/anima"
        )))
    }

    /// A resolver whose own text looks like async-graphql's coercion text (COULD 1).
    async fn lookalike(&self, kind: Option<i32>) -> async_graphql::Result<bool> {
        let prefix = if kind.is_some() {
            r#"Expected input type "Disk""#
        } else {
            r#"Failed to parse "Disk""#
        };
        Err(async_graphql::Error::new(format!(
            "{prefix}: /dev/sda1 failed with token={SECRET}"
        )))
    }
}

pub(super) type TestSchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// Requests whose argument async-graphql cannot coerce while resolving, each holding `SECRET`,
/// with the argument's position and name.
fn malformed() -> [(Request, Pos, &'static str); 4] {
    let variables = |input| Variables::from_json(json!({ "input": input }));
    let at = |column| Pos { line: 1, column };
    [
        (
            Request::new("query($input: SignIn!) { login(input: $input) }")
                .variables(variables(json!(SECRET))),
            at(39),
            "input",
        ),
        (
            Request::new("query($input: Proof!) { prove(proof: $input) }")
                .variables(variables(json!(SECRET))),
            at(38),
            "proof",
        ),
        (
            Request::new(format!(r#"{{ unlock(pin: "{SECRET}") }}"#)),
            at(15),
            "pin",
        ),
        (
            Request::new(format!(r#"{{ coded(pin: "{SECRET}") }}"#)),
            at(14),
            "pin",
        ),
    ]
}

fn plain() -> TestSchema {
    Schema::new(Query, EmptyMutation, EmptySubscription)
}

pub(super) fn hardened() -> TestSchema {
    harden(Schema::build(Query, EmptyMutation, EmptySubscription), true).finish()
}

/// Pins async-graphql's own shape: an upgrade that changes it fails here, not in production.
/// A coercion error is placed at the argument's value and echoes it.
#[tokio::test]
async fn log_007_async_graphql_places_a_bad_argument_at_its_value_and_echoes_it() {
    for (request, at, _) in malformed() {
        let response: Response = plain().execute(request).await;
        assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
        let error = &response.errors[0];
        assert!(!error.path.is_empty(), "{error:?}");
        assert_eq!(error.locations, [at], "{error:?}");
        assert!(error.message.contains(SECRET), "{error:?}");
    }
}

#[tokio::test]
async fn log_007_a_bad_argument_is_validation_naming_it_and_its_value_stays_out_of_the_log() {
    let (capture, _guard) = capture_log("trace");

    for (request, _, argument) in malformed() {
        let response = hardened().execute(request).await;
        let error = serde_json::to_value(&response.errors[0]).unwrap();
        assert_eq!(error["message"], "An argument has the wrong type or shape.");
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
        assert_eq!(
            error["extensions"]["fields"],
            json!([{ "field": argument, "message": "An argument has the wrong type or shape." }]),
        );
        assert!(!error.to_string().contains(SECRET), "{error}");
    }

    let text = capture.text();
    assert_eq!(text.matches("argument refused").count(), 4, "{text}");
    assert!(!text.contains(SECRET), "{text}");
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

#[tokio::test]
async fn api_010_a_resolver_error_that_reads_like_a_coercion_error_stays_internal_and_is_logged() {
    let (capture, _guard) = capture_log("trace");

    for (query, prefix) in [
        ("{ lookalike }", "Failed to parse"),
        ("{ lookalike(kind: 1) }", "Expected input type"),
    ] {
        let response = hardened().execute(query).await;
        let error = serde_json::to_value(&response.errors[0]).unwrap();
        assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
        assert_eq!(error["message"], "Something went wrong on the hub.");
        let text = capture.text();
        assert!(text.contains(prefix), "{text}");
        assert!(text.contains("/dev/sda1 failed"), "{text}");
        assert!(!text.contains(SECRET), "{text}");
    }
    assert!(!capture.text().contains("argument refused"));
}
