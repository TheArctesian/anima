//! The root rule of each token kind in the backstop: an API key may not select a field on the
//! D-23 list or any root fragment, in any operation of a document, and a read key runs no
//! mutation. Each refusal is one FORBIDDEN in `parse_query`, before validation and before any
//! resolver (D-23, WP-43 A-6, R-084, CR-8).

use async_graphql::{Request, Response, Value, value};
use chrono::TimeDelta;

use super::{backstopped, identity_of, now};
use crate::access::{Scope, TokenKind, guard::kind_may_select};

const KIND_REFUSED: &str = "This token cannot do that. Sign in to the hub to do it.";

/// The D-23 list as the spec and A-6 state it, with the CR-8 agent fields.
const D_23: [&str; 18] = [
    "createApiKey",
    "revokeApiKey",
    "apiKeys",
    "signInBrowser",
    "logout",
    "startSession",
    "openFiles",
    "shareSession",
    "joinShare",
    "openRecording",
    "setSshCredential",
    "clearSshCredential",
    "setRdpCredential",
    "clearRdpCredential",
    "setVncCredential",
    "clearVncCredential",
    "enrolAgent",
    "revokeAgentToken",
];

async fn run(kind: TokenKind, scope: Scope, request: Request) -> Response {
    let identity = identity_of(kind, scope, now() + TimeDelta::days(1));
    backstopped().execute(identity.attach(request)).await
}

/// A request for `document` that runs `operation` when one is named.
fn named(document: &str, operation: Option<&str>) -> Request {
    match operation {
        Some(operation) => Request::new(document).operation_name(operation),
        None => Request::new(document),
    }
}

fn assert_refused(response: &Response, message: &str, document: &str) {
    assert_eq!(response.data, Value::Null, "{document}");
    let [error] = response.errors.as_slice() else {
        panic!("{document}: expected one error, got {:?}", response.errors);
    };
    assert_eq!(error.message, message, "{document}");
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    assert_eq!(code, Some(&value!("FORBIDDEN")), "{document}");
}

#[test]
fn d_23_only_an_api_key_is_kept_from_the_listed_roots() {
    for name in D_23 {
        assert!(!kind_may_select(TokenKind::ApiKey, name), "{name}");
        assert!(kind_may_select(TokenKind::Login, name), "{name}");
        assert!(kind_may_select(TokenKind::Browser, name), "{name}");
    }
    for name in ["me", "devices", "createSite", "login", "__typename"] {
        assert!(kind_may_select(TokenKind::ApiKey, name), "{name}");
    }
}

#[tokio::test]
async fn d_23_an_api_key_is_refused_each_listed_root_in_any_spelling() {
    for name in D_23 {
        let documents = [
            (format!("mutation {{ {name} }}"), None),
            (format!("{{ {name} }}"), None),
            (format!("mutation {{ a: {name} b: open }}"), None),
            (format!("mutation {{ ... on Mutation {{ {name} }} }}"), None),
            (
                format!("mutation {{ ...F }} fragment F on Mutation {{ {name} }}"),
                None,
            ),
            (
                format!("query A {{ open }} mutation B {{ {name} }}"),
                Some("A"),
            ),
            (format!("subscription {{ {name} }}"), None),
        ];
        for scope in [Scope::Read, Scope::Write] {
            for (document, operation) in &documents {
                let response = run(TokenKind::ApiKey, scope, named(document, *operation)).await;
                assert_refused(&response, KIND_REFUSED, document);
            }
        }
    }
}

#[tokio::test]
async fn d_23_an_api_key_is_refused_any_root_fragment() {
    for document in [
        "{ ...F } fragment F on Query { open }",
        "{ ... on Query { open } }",
        "{ ... { open } }",
        "mutation { ... on Mutation { open } }",
    ] {
        let response = run(TokenKind::ApiKey, Scope::Write, Request::new(document)).await;
        assert_refused(&response, KIND_REFUSED, document);
    }
}

#[tokio::test]
async fn d_23_login_and_browser_tokens_keep_every_root() {
    for kind in [TokenKind::Login, TokenKind::Browser] {
        let response = run(
            kind,
            Scope::Write,
            Request::new("mutation { signInBrowser }"),
        )
        .await;
        assert_eq!(
            response.data,
            value!({ "signInBrowser": "browser signed in" })
        );
        let fragment = Request::new("{ ...F } fragment F on Query { open }");
        let response = run(kind, Scope::Write, fragment).await;
        assert_eq!(response.data, value!({ "open": "open" }), "{kind:?}");
    }
}

#[tokio::test]
async fn d_23_a_write_key_runs_other_fields() {
    let mutation = run(
        TokenKind::ApiKey,
        Scope::Write,
        Request::new("mutation { open }"),
    )
    .await;
    assert_eq!(mutation.data, value!({ "open": "open" }));
    let query = run(TokenKind::ApiKey, Scope::Write, Request::new("{ open me }")).await;
    assert_eq!(query.data, value!({ "open": "open", "me": "operator" }));
}

#[tokio::test]
async fn d_23_a_read_key_runs_no_mutation_at_all() {
    for document in [
        "mutation { open }",
        "mutation { login }",
        "mutation { open __typename }",
        "query A { open } mutation B { login }",
    ] {
        let response = run(
            TokenKind::ApiKey,
            Scope::Read,
            Request::new(document).operation_name("A"),
        )
        .await;
        assert_refused(&response, "This token may only read.", document);
    }
    let typename = run(
        TokenKind::ApiKey,
        Scope::Read,
        Request::new("mutation { __typename }"),
    )
    .await;
    assert_eq!(typename.data, value!({ "__typename": "Mutation" }));
}
