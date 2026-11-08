//! The real `access` fields keep their own guard, and a `read` token cannot mutate (API-003).

use async_graphql::{EmptySubscription, Request, Schema, value};
use chrono::TimeDelta;

use super::{assert_unauthenticated, backstopped, clock, identity_of, now, operator};
use crate::access::{Identity, Scope, TokenKind, graphql};

fn read_identity() -> Identity {
    identity_of(TokenKind::ApiKey, Scope::Read, now() + TimeDelta::days(1))
}

#[tokio::test]
async fn api_003_access_fields_refuse_without_an_identity_even_without_the_backstop() {
    let schema = Schema::build(graphql::Query, graphql::Mutation, EmptySubscription)
        .data(clock(now()))
        .data(operator())
        .finish();
    assert_unauthenticated(&schema.execute("{ me { id } }").await);
    assert_unauthenticated(&schema.execute("mutation { logout }").await);
}

#[tokio::test]
async fn api_003_a_read_token_cannot_run_a_mutation() {
    let response = backstopped()
        .execute(read_identity().attach(Request::new("mutation { open }")))
        .await;
    let [error] = response.errors.as_slice() else {
        panic!("expected one error, got {:?}", response.errors);
    };
    assert_eq!(error.message, "This token may only read.");
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    assert_eq!(code, Some(&value!("FORBIDDEN")));
}

#[tokio::test]
async fn api_003_a_read_token_may_query_and_sign_out() {
    let query = backstopped()
        .execute(read_identity().attach(Request::new("{ open me }")))
        .await;
    assert_eq!(query.data, value!({ "open": "open", "me": "operator" }));
    let login = backstopped()
        .execute(read_identity().attach(Request::new("mutation { login __typename }")))
        .await;
    assert!(login.errors.is_empty(), "{:?}", login.errors);
}

#[tokio::test]
async fn api_003_a_read_token_may_query_through_a_root_fragment() {
    let document = "query { ...F } fragment F on Query { open }";
    let response = backstopped()
        .execute(read_identity().attach(Request::new(document)))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(response.data, value!({ "open": "open" }));
}

#[tokio::test]
async fn api_003_a_read_token_cannot_mutate_through_a_fragment() {
    for document in [
        "mutation { ...F } fragment F on Mutation { open }",
        "mutation { ... on Mutation { open } }",
    ] {
        let response = backstopped()
            .execute(read_identity().attach(Request::new(document)))
            .await;
        let codes: Vec<_> = response
            .errors
            .iter()
            .map(|error| {
                error
                    .extensions
                    .as_ref()
                    .and_then(|e| e.get("code"))
                    .cloned()
            })
            .collect();
        assert_eq!(codes, [Some(value!("FORBIDDEN"))], "{document}");
        assert_eq!(response.data, value!(null), "{document}");
    }
}
