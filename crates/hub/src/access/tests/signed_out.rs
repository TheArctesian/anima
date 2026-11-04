//! Signed out, validation errors are not shown: they would map the schema (T3 security
//! SHOULD 1). Signed in, they are. A guard without a clock writes its fault to the log.

use async_graphql::{EmptySubscription, Object, Request, Schema};
use chrono::TimeDelta;

use super::{Mutation, Query, assert_unauthenticated, backstopped, clock, identity, now};
use crate::{
    access::SignInRequired, events::process_log::tests::capture::capture_log, schema::harden,
};

/// A `login` that fails inside the hub, with no code: a fault, not a refusal.
struct FaultyLogin;

#[expect(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    reason = "async-graphql resolvers must be async"
)]
#[Object]
impl FaultyLogin {
    async fn login(&self) -> async_graphql::Result<bool> {
        Err(async_graphql::Error::new("the disk is full"))
    }
}

#[tokio::test]
async fn api_010_signed_out_a_resolver_fault_is_internal_and_logged_not_a_refusal() {
    let (capture, _guard) = capture_log("trace");
    let builder = Schema::build(Query, FaultyLogin, EmptySubscription).data(clock(now()));
    let schema = harden(builder, true).extension(SignInRequired).finish();

    let response = schema.execute("mutation { login }").await;

    let [error] = response.errors.as_slice() else {
        panic!("one error: {:?}", response.errors);
    };
    assert_eq!(error.message, "Something went wrong on the hub.");
    assert!(capture.text().contains("the disk is full"));
}

/// Documents the root check allows whose validation fails, each naming a schema part.
const MAPPING: [&str; 4] = [
    "{ __typename } fragment F on Query { nosuch }",
    "{ __typename } fragment F on NoSuchType { id }",
    "query($x: NoSuchType) { __typename }",
    "query @nosuch { __typename }",
];

#[tokio::test]
async fn api_003_signed_out_a_failed_validation_is_one_unauthenticated() {
    for document in MAPPING {
        assert_unauthenticated(&backstopped().execute(document).await);
    }
    let login_with_fragment = "mutation { login } fragment F on Query { nosuch }";
    assert_unauthenticated(&backstopped().execute(login_with_fragment).await);
}

/// A request whose error has neither a path nor a code: a syntax error, or an
/// `operationName` the document lacks, named like a secret.
fn unexplained() -> [Request; 2] {
    [
        Request::new("{ __typename "),
        Request::new("query A { __typename }").operation_name("hunter2-correct-horse"),
    ]
}

#[tokio::test]
async fn log_007_signed_out_an_error_without_a_path_or_code_is_one_unauthenticated() {
    for request in unexplained() {
        assert_unauthenticated(&backstopped().execute(request).await);
    }
}

#[tokio::test]
async fn log_007_signed_in_an_error_without_a_path_or_code_is_kept() {
    for request in unexplained() {
        let request = identity(now() + TimeDelta::days(1)).attach(request);
        let response = backstopped().execute(request).await;
        assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
        assert_ne!(response.errors[0].message, "Sign in to continue.");
    }
}

#[tokio::test]
async fn api_003_signed_in_validation_errors_are_shown() {
    for document in MAPPING {
        let request = identity(now() + TimeDelta::days(1)).attach(Request::new(document));
        let response = backstopped().execute(request).await;
        assert!(!response.errors.is_empty(), "{document}");
        for error in &response.errors {
            assert_ne!(error.message, "Sign in to continue.", "{document}");
        }
    }
}

#[tokio::test]
async fn log_007_the_guard_logs_a_schema_without_a_clock() {
    let (capture, _guard) = capture_log("trace");
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
    let request = identity(now() + TimeDelta::days(1)).attach(Request::new("{ me }"));

    let response = schema.execute(request).await;

    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    let lines = capture.lines();
    let [line] = lines.as_slice() else {
        panic!("one log line: {lines:?}");
    };
    assert_eq!(line["level"], "ERROR");
    assert_eq!(line["fields"]["message"], "access request failed");
    assert_eq!(line["fields"]["fault"], "missing_context");
}
