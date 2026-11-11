//! The root rule of each token kind in the backstop: an API key may not select a field on the
//! D-23 list or any root fragment, in any operation of a document, and a read key runs no
//! mutation. Each refusal is one FORBIDDEN in `parse_query`, before validation and before any
//! resolver (D-23, WP-43 A-6, R-084, CR-8, R-100).

use async_graphql::{EmptySubscription, Request, Response, Schema, Value, value};
use chrono::TimeDelta;

use super::{backstopped, clock, identity_of, now};
use crate::access::{
    Scope, TokenKind,
    graphql::{Mutation, Query},
    guard::{API_KEY_REFUSED_ROOTS, BUILDER_FIELDS, kind_may_select},
};

const KIND_REFUSED: &str = "This token cannot do that. Sign in to the hub to do it.";

/// The D-23 list as the spec and A-6 state it, with the CR-8 agent fields, the R-100
/// device identity, device and recording fields, the R-107 builder and release fields, and the
/// VP-7 VPN key, registration and renewal fields.
const D_23: [&str; 32] = [
    "createApiKey",
    "revokeApiKey",
    "apiKeys",
    "signInBrowser",
    "logout",
    "startSession",
    "openFiles",
    "openRecording",
    "setSshCredential",
    "clearSshCredential",
    "setRdpCredential",
    "clearRdpCredential",
    "setVncCredential",
    "clearVncCredential",
    "enrolAgent",
    "revokeAgentToken",
    "acceptHostKey",
    "acceptServerCertificate",
    "acceptWebCertificate",
    "updateDevice",
    "deleteDevice",
    "setRetention",
    "deleteRecording",
    "enrolBuilder",
    "revokeBuilder",
    "createRelease",
    "cancelBuildJob",
    "createVpnInvite",
    "createRouterKey",
    "registerVpnNode",
    "renewVpnKey",
    "approveSiteRoute",
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
    let open = [
        "me",
        "devices",
        "createSite",
        "createDevice",
        "testConnection",
        "createNotificationChannel",
        "login",
        "__typename",
    ];
    for name in open {
        assert!(kind_may_select(TokenKind::ApiKey, name), "{name}");
    }
}

#[test]
fn r_102_a_builder_token_may_select_only_the_builder_mutations() {
    for name in BUILDER_FIELDS.into_iter().chain(["__typename"]) {
        assert!(kind_may_select(TokenKind::Builder, name), "{name}");
    }
    for name in [
        "me",
        "builders",
        "enrolBuilder",
        "createRelease",
        "logout",
        "__schema",
    ] {
        assert!(!kind_may_select(TokenKind::Builder, name), "{name}");
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

/// The root fields of K1 with their doc strings, `@pending` ones included.
fn contract_roots() -> Vec<(String, String)> {
    use async_graphql::parser::{
        parse_schema,
        types::{TypeKind, TypeSystemDefinition},
    };
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/schema.graphql"
    );
    let text = std::fs::read_to_string(path).unwrap();
    let mut roots = Vec::new();
    for definition in parse_schema(&text).unwrap().definitions {
        let TypeSystemDefinition::Type(declared) = definition else {
            continue;
        };
        let root =
            ["Query", "Mutation", "Subscription"].contains(&declared.node.name.node.as_str());
        if let (true, TypeKind::Object(object)) = (root, &declared.node.kind) {
            roots.extend(object.fields.iter().map(|field| {
                let doc = field.node.description.as_ref();
                let doc = doc.map(|doc| doc.node.clone()).unwrap_or_default();
                (field.node.name.node.to_string(), doc)
            }));
        }
    }
    roots
}

/// A name on the list that K1 does not declare would refuse nothing, so a renamed field would
/// slip past the rule (D-23, R-100).
#[test]
fn d_23_every_refused_name_is_a_root_field_of_k1() {
    let roots = contract_roots();
    for name in API_KEY_REFUSED_ROOTS {
        assert!(
            roots.iter().any(|(root, _)| root == name),
            "{name} is not a K1 root field"
        );
    }
    assert_eq!(D_23, API_KEY_REFUSED_ROOTS);
}

/// The other direction: a K1 root documented as closed to API keys is on the list, so the
/// contract's doc string and the guard cannot drift apart (D-23, R-100).
#[test]
fn d_23_every_k1_root_documented_closed_to_keys_is_listed() {
    let closed: Vec<String> = contract_roots()
        .into_iter()
        .filter(|(_, doc)| {
            doc.contains("Login or browser token only") || doc.contains("never an API key")
        })
        .map(|(name, _)| name)
        .collect();
    for name in &closed {
        assert!(
            API_KEY_REFUSED_ROOTS.contains(&name.as_str()),
            "{name} is documented closed to API keys but not listed"
        );
    }
    assert_eq!(closed.len(), API_KEY_REFUSED_ROOTS.len(), "{closed:?}");
}

/// The key fields' own resolvers on a schema built without the backstop: the hub's roots with a
/// clock and nothing else, so any resolver that got past its kind check would fail on the
/// missing database instead.
async fn unguarded(kind: TokenKind, document: &str) -> Response {
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(clock(now()))
        .finish();
    let identity = identity_of(kind, Scope::Write, now() + TimeDelta::days(1));
    schema
        .execute(identity.attach(Request::new(document)))
        .await
}

const KEY_DOCUMENTS: [&str; 3] = [
    "{ apiKeys { id } }",
    "mutation { createApiKey(input: { name: \"x\", scope: WRITE, \
     expiresAt: \"2030-06-01T00:00:00Z\" }) { secret } }",
    "mutation { revokeApiKey(id: \"0190a000-0000-7000-8000-0000000000aa\") }",
];

/// The resolvers check the token kind again, so a schema without `SignInRequired` still keeps
/// an API key from managing keys (D-23).
#[tokio::test]
async fn d_23_the_key_resolvers_refuse_an_api_key_without_the_backstop() {
    for document in KEY_DOCUMENTS {
        let response = unguarded(TokenKind::ApiKey, document).await;
        assert_refused(&response, KIND_REFUSED, document);
    }
    for document in KEY_DOCUMENTS {
        let response = unguarded(TokenKind::Login, document).await;
        let [error] = response.errors.as_slice() else {
            panic!("{document}: expected one error, got {:?}", response.errors);
        };
        assert_ne!(
            error.message, KIND_REFUSED,
            "a login token passes: {document}"
        );
    }
}
