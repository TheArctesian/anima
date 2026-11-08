//! The guard and the API-003 backstop, without a database. Rules are in `tests/rules.rs`;
//! the HTTP and database behaviour is in the `it` crate (`tests/it/login.rs`, `tokens.rs`).

use std::{io::Write as _, sync::Arc};

use async_graphql::{
    EmptySubscription, Object, Request, Response, Schema, SchemaBuilder, Value, value,
};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use uuid::Uuid;

use super::{Authenticated, Identity, Operator, Scope, SignInRequired, TokenKind, identity::Grant};
use crate::{
    clock::{Clock, FixedClock},
    credentials::Vault,
};

mod kinds;
mod networks;
mod races;
mod rules;
mod runtime;
mod scope;
mod second_factor;
mod signed_out;
mod timing;
mod totp;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

/// A vault over a throwaway key of `fill` bytes in a temporary file; never a real key
/// file (SEC-004).
fn test_vault(fill: u8) -> Vault {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(key_file, "{}", STANDARD.encode([fill; 32])).unwrap();
    Vault::from_key_file(key_file.path()).unwrap()
}

fn operator() -> Operator {
    Operator {
        id: "0190a000-0000-7000-8000-000000000001".parse().unwrap(),
        name: "operator".to_owned(),
    }
}

fn identity(expires_at: DateTime<Utc>) -> Identity {
    identity_of(TokenKind::Login, Scope::Write, expires_at)
}

fn identity_of(kind: TokenKind, scope: Scope, expires_at: DateTime<Utc>) -> Identity {
    let grant = Grant {
        token: Uuid::now_v7(),
        kind,
        scope,
        expires_at,
    };
    Identity::new(operator(), grant)
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

    /// An unguarded field whose argument never parses, so any argument error would show.
    async fn probe(&self, value: Refusing) -> &'static str {
        let _ = value;
        "probed"
    }
}

/// A scalar that refuses every value with a message naming its rule.
struct Refusing;

#[async_graphql::Scalar]
impl async_graphql::ScalarType for Refusing {
    fn parse(_: Value) -> async_graphql::InputValueResult<Self> {
        Err(async_graphql::InputValueError::custom(
            "the secret argument rule",
        ))
    }

    fn to_value(&self) -> Value {
        Value::Null
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn login(&self) -> &'static str {
        "signed in"
    }

    async fn sign_in_browser(&self) -> &'static str {
        "browser signed in"
    }

    async fn join_share(&self) -> &'static str {
        "joined"
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
    let response = schema.execute(request).await;
    assert_eq!(response.data, Value::Null, "{:?}", response.data);
    let [error] = response.errors.as_slice() else {
        panic!("expected one error, got {:?}", response.errors);
    };
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    assert_eq!(code, Some(&value!("INTERNAL")), "{error:?}");
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
async fn api_003_backstop_lets_one_login_or_typename_through_without_an_identity() {
    let login = backstopped().execute("mutation { login }").await;
    assert_eq!(login.data, value!({ "login": "signed in" }));
    let browser = backstopped().execute("mutation { signInBrowser }").await;
    assert_eq!(
        browser.data,
        value!({ "signInBrowser": "browser signed in" })
    );
    let share = backstopped().execute("mutation { joinShare }").await;
    assert_eq!(share.data, value!({ "joinShare": "joined" }));
    let renamed = backstopped().execute("mutation { signIn: login }").await;
    assert_eq!(renamed.data, value!({ "signIn": "signed in" }));
    let typename = backstopped().execute("{ __typename a: __typename }").await;
    assert_eq!(
        typename.data,
        value!({ "__typename": "Query", "a": "Query" })
    );
}

#[tokio::test]
async fn api_003_h_6_signed_out_bad_arguments_get_only_unauthenticated() {
    let documents = [
        r#"{ probe(value: "anything") }"#,
        "{ probe(value: 42) }",
        "query($v: Refusing!) { probe(value: $v) }",
        "{ probe }",
        "{ nosuchfield(x: 1) }",
        "{ login }",
    ];
    for document in documents {
        let request = Request::new(document).variables(async_graphql::Variables::from_json(
            serde_json::json!({ "v": "anything" }),
        ));
        assert_unauthenticated(&backstopped().execute(request).await);
    }
}

#[tokio::test]
async fn api_003_backstop_refuses_mixed_fragment_and_introspection_documents() {
    let documents = [
        "mutation { login __typename }",
        "mutation { a0: login a1: login }",
        "mutation { signInBrowser login }",
        "mutation { login joinShare }",
        "mutation { a: signInBrowser b: signInBrowser }",
        "mutation { signInBrowser open }",
        "mutation { signInBrowser __typename }",
        "mutation { ... on Mutation { signInBrowser } }",
        "mutation A { signInBrowser } query B { open }",
        "{ signInBrowser }",
        "{ joinShare }",
        "mutation A { __typename } mutation B { login open }",
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
