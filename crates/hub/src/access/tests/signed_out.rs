//! Signed out, validation errors are not shown: they would map the schema (T3 security
//! SHOULD 1). Signed in, they are. A guard without a clock writes its fault to the log.

use async_graphql::{EmptySubscription, Request, Schema};
use chrono::TimeDelta;

use super::{Mutation, Query, assert_unauthenticated, backstopped, identity, now};
use crate::events::process_log::tests::capture::capture_log;

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
