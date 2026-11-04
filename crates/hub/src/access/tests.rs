//! The guard and the API-003 backstop, without a database. Rules are in `tests/rules.rs`;
//! the HTTP and database behaviour is in the `it` crate (`tests/it/login.rs`, `tokens.rs`).

use std::sync::Arc;

use async_graphql::{
    EmptySubscription, Object, Request, Response, Schema, SchemaBuilder, Value, value,
};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use uuid::Uuid;

use super::{Authenticated, Identity, Operator, Scope, SignInRequired};
use crate::clock::{Clock, FixedClock};

mod rules;
mod runtime;
mod scope;
mod timing;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

fn operator() -> Operator {
    Operator {
        id: "0190a000-0000-7000-8000-000000000001".parse().unwrap(),
        name: "operator".to_owned(),
    }
}

fn identity(expires_at: DateTime<Utc>) -> Identity {
    Identity::new(operator(), Uuid::now_v7(), Scope::Write, expires_at)
}

fn clock(at: DateTime<Utc>) -> Arc<dyn Clock> {
    Arc::new(FixedClock(at))
}

struct Query;

#[Object]
impl Query {
    #[graphql(guard = "Authenticated")]
    async fn me(&self, ctx: &async_graphql::Context<'_>) -> String {
        ctx.data_unchecked::<Operator>().name.clone()
    }

    /// A resolver that forgot its guard: only the backstop protects it.
    async fn open(&self) -> &'static str {
        "open"
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn login(&self) -> &'static str {
        "signed in"
    }

    async fn open(&self) -> &'static str {
        "open"
    }
}

type TestSchema = Schema<Query, Mutation, EmptySubscription>;

fn builder() -> SchemaBuilder<Query, Mutation, EmptySubscription> {
    Schema::build(Query, Mutation, EmptySubscription).data(clock(now()))
}

/// The guard alone, without the backstop.
fn guarded() -> TestSchema {
    builder().finish()
}

fn backstopped() -> TestSchema {
    builder().extension(SignInRequired).finish()
}

fn assert_unauthenticated(response: &Response) {
    assert_eq!(response.data, Value::Null, "{:?}", response.data);
    let [error] = response.errors.as_slice() else {
        panic!("expected one error, got {:?}", response.errors);
    };
    assert_eq!(error.message, "Sign in to continue.");
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    assert_eq!(code, Some(&value!("UNAUTHENTICATED")));
}

#[tokio::test]
async fn api_003_guard_refuses_a_request_without_an_identity() {
    assert_unauthenticated(&guarded().execute("{ me }").await);
}

#[tokio::test]
async fn api_003_guard_refuses_an_operator_without_an_identity() {
    let schema = builder().data(operator()).finish();
    assert_unauthenticated(&schema.execute("{ me }").await);
    let request = Request::new("{ me }").data(operator());
    assert_unauthenticated(&guarded().execute(request).await);
}

#[tokio::test]
async fn api_003_guard_passes_a_request_with_an_identity() {
    let request = identity(now() + TimeDelta::days(1)).attach(Request::new("{ me }"));
    let response = guarded().execute(request).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(response.data, value!({ "me": "operator" }));
}

#[tokio::test]
async fn api_003_guard_refuses_an_identity_at_its_expiry() {
    let request = identity(now()).attach(Request::new("{ me }"));
    assert_unauthenticated(&guarded().execute(request).await);
    let just_before = identity(now() + TimeDelta::seconds(1)).attach(Request::new("{ me }"));
    let response = guarded().execute(just_before).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
}

#[tokio::test]
async fn api_003_guard_fails_closed_without_a_clock() {
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
    let request = identity(now() + TimeDelta::days(1)).attach(Request::new("{ me }"));
    assert_unauthenticated(&schema.execute(request).await);
}

#[tokio::test]
async fn api_003_backstop_refuses_an_unguarded_field_without_an_identity() {
    assert_unauthenticated(&backstopped().execute("{ open }").await);
    assert_unauthenticated(&backstopped().execute("mutation { open }").await);
    let with_operator = Request::new("{ open }").data(operator());
    assert_unauthenticated(&backstopped().execute(with_operator).await);
    // Without the backstop the same field answers: the test above proves the extension.
    assert_eq!(
        guarded().execute("{ open }").await.data,
        value!({ "open": "open" })
    );
}

#[tokio::test]
async fn api_003_backstop_lets_exactly_one_login_through_without_an_identity() {
    let login = backstopped().execute("mutation { login }").await;
    assert_eq!(login.data, value!({ "login": "signed in" }));
    let renamed = backstopped().execute("mutation { signIn: login }").await;
    assert_eq!(renamed.data, value!({ "signIn": "signed in" }));
}

#[tokio::test]
async fn api_003_backstop_refuses_several_logins_as_validation() {
    let response = backstopped()
        .execute("mutation { a0: login a1: login a2: login }")
        .await;
    assert_eq!(response.data, Value::Null);
    let [error] = response.errors.as_slice() else {
        panic!("expected one error, got {:?}", response.errors);
    };
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    assert_eq!(code, Some(&value!("VALIDATION")));
}

#[tokio::test]
async fn api_003_backstop_refuses_mixed_fragment_and_introspection_documents() {
    let documents = [
        "{ __typename }",
        "mutation { login __typename }",
        "mutation { login open }",
        "mutation { renamed: open }",
        "mutation { ...F } fragment F on Mutation { login }",
        "mutation { ... on Mutation { login } }",
        "{ __schema { types { name } } }",
        "{ __type(name: \"Query\") { name } }",
        "mutation A { login } query B { open }",
    ];
    for document in documents {
        let request = Request::new(document).operation_name("A");
        let response = backstopped().execute(request).await;
        assert_unauthenticated(&response);
    }
}

#[tokio::test]
async fn api_003_backstop_passes_a_valid_identity_and_refuses_an_expired_one() {
    let valid = identity(now() + TimeDelta::days(1)).attach(Request::new("{ open }"));
    assert_eq!(
        backstopped().execute(valid).await.data,
        value!({ "open": "open" })
    );
    let expired = identity(now()).attach(Request::new("{ open }"));
    assert_unauthenticated(&backstopped().execute(expired).await);
}

#[tokio::test]
async fn api_003_connection_data_carries_the_identity_and_operator() {
    use async_graphql::futures_util::StreamExt as _;

    let data = Arc::new(identity(now() + TimeDelta::days(1)).into_connection_data());
    let mut stream = backstopped().execute_stream_with_session_data("{ me }", data);
    let response = stream.next().await.expect("one response");
    assert_eq!(response.data, value!({ "me": "operator" }));
}
