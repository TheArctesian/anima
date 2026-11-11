//! What an API key may not do: the D-23 root fields, by any spelling of the document, and any
//! mutation with a READ key. Each is refused with FORBIDDEN before any resolver runs (D-23,
//! WP-43 A-6, R-084, CR-8, R-100, API-003).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{CREATE, ME, REVOKE, count, input, kind_refused, made, signed_in};
use crate::hub::{Hub, PASSWORD, USERNAME, error};

/// Every root field an API key may never select (D-23, A-6; `enrolAgent` and
/// `revokeAgentToken` by name before they have resolvers, R-084, CR-8; pinning a device
/// identity, moving or deleting a device and destroying recordings, R-100; VPN keys,
/// registration and renewal, VP-7).
const REFUSED: [&str; 31] = [
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
];

/// Documents that select `name` at the root: plainly, through an alias, an inline fragment, a
/// fragment spread, beside other fields, with arguments, and in an operation of a
/// multi-operation document that is not the one named to run. Each with its operation name.
fn spellings(name: &str) -> Vec<(String, Option<&'static str>)> {
    let (keyword, root) = if name == "apiKeys" {
        ("query", "Query")
    } else {
        ("mutation", "Mutation")
    };
    vec![
        (format!("{keyword} {{ {name} }}"), None),
        (format!("{keyword} {{ renamed: {name} }}"), None),
        (format!("{keyword} {{ ... on {root} {{ {name} }} }}"), None),
        (
            format!("{keyword} {{ ...F }} fragment F on {root} {{ {name} }}"),
            None,
        ),
        (
            format!("{keyword} {{ __typename {name}(id: \"x\") {{ id }} }}"),
            None,
        ),
        (
            format!("query A {{ __typename }} {keyword} B {{ {name} }}"),
            Some("A"),
        ),
        (
            format!("{keyword} B {{ x: {name} }} query A {{ __typename }}"),
            Some("B"),
        ),
        (format!("query {{ {name} }}"), None),
        (format!("subscription {{ {name} }}"), None),
    ]
}

/// Posts `document` with `token`, naming the operation `operation` to run.
async fn post_named(hub: &Hub, token: &str, document: &str, operation: Option<&str>) -> Value {
    let mut body = json!({ "query": document, "variables": {} });
    if let Some(operation) = operation {
        body["operationName"] = json!(operation);
    }
    let request = axum::http::Request::post("/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(axum::body::Body::from(body.to_string()))
        .unwrap();
    let (status, text) = hub.send(request).await;
    assert!(status.is_success(), "{status} {text}");
    serde_json::from_str(&text).unwrap()
}

/// The row counts a resolver of the D-23 list would change.
async fn footprint(pool: &PgPool) -> Vec<i64> {
    let mut counts = Vec::new();
    for table in [
        "access.api_tokens",
        "access.login_attempts",
        "events.audit_log",
        "events.event_log",
        "sessions.sessions",
    ] {
        counts.push(count(pool, table).await);
    }
    counts
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_every_refused_root_is_forbidden_for_an_api_key_in_every_spelling(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    for scope in ["READ", "WRITE"] {
        let key = made(&hub, &token, scope, scope).await;
        let secret = key["secret"].as_str().unwrap();
        let before = footprint(&pool).await;
        for name in REFUSED {
            for (document, operation) in spellings(name) {
                let response = post_named(&hub, secret, &document, operation).await;
                assert_eq!(error(&response), kind_refused(), "{scope} {document}");
                assert_eq!(response["data"], Value::Null, "{scope} {document}");
            }
        }
        assert_eq!(footprint(&pool).await, before, "{scope}: no resolver ran");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_refused_fields_with_valid_arguments_change_nothing(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let key = made(&hub, &token, "writer", "WRITE").await;
    let secret = key["secret"].as_str().unwrap();
    let id = key["key"]["id"].clone();
    let sign_in = "mutation($input: LoginInput!) { signInBrowser(input: $input) { expiresAt } }";
    let credentials = json!({ "input": { "username": USERNAME, "password": PASSWORD } });
    let before = footprint(&pool).await;
    let documents = [
        (CREATE, json!({ "input": input("another", "WRITE") })),
        (REVOKE, json!({ "id": id })),
        ("mutation { logout }", json!({})),
        ("{ apiKeys { id name } }", json!({})),
        (sign_in, credentials),
    ];
    for (document, variables) in documents {
        let response = hub.post(Some(secret), document, variables).await;
        assert_eq!(error(&response), kind_refused(), "{document}");
    }
    assert_eq!(footprint(&pool).await, before);
    let me = hub.post(Some(secret), ME, json!({})).await;
    assert_eq!(
        me["data"]["me"]["username"], USERNAME,
        "the key still works"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_an_api_key_cannot_select_any_root_fragment(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let key = made(&hub, &token, "reader", "READ").await;
    let secret = key["secret"].as_str().unwrap();
    let documents = [
        "{ ...F } fragment F on Query { me { username } }",
        "{ ... on Query { me { username } } }",
        "{ ... @include(if: true) { me { username } } }",
    ];
    for document in documents {
        let refused = hub.post(Some(secret), document, json!({})).await;
        assert_eq!(error(&refused), kind_refused(), "{document}");
        let login = hub.post(Some(&token), document, json!({})).await;
        assert_eq!(
            login["data"]["me"]["username"], USERNAME,
            "a login token may: {document}"
        );
    }
}

/// The root mutation fields of the hub schema, by introspection with `token`.
async fn mutation_fields(hub: &Hub, token: &str) -> Vec<String> {
    let query = "{ __schema { mutationType { fields { name } } } }";
    let schema = hub.post(Some(token), query, json!({})).await;
    let fields = schema["data"]["__schema"]["mutationType"]["fields"]
        .as_array()
        .unwrap();
    fields
        .iter()
        .map(|field| field["name"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_read_key_cannot_run_any_mutation(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let key = made(&hub, &token, "reader", "READ").await;
    let secret = key["secret"].as_str().unwrap();
    let fields = mutation_fields(&hub, &token).await;
    assert!(fields.len() > 10, "{fields:?}");
    let before = footprint(&pool).await;
    for name in &fields {
        let response = hub
            .post(Some(secret), &format!("mutation {{ {name} }}"), json!({}))
            .await;
        assert_eq!(error(&response).1, "FORBIDDEN", "{name}: {response}");
    }
    let login = "mutation($input: LoginInput!) { login(input: $input) { token } }";
    let credentials = json!({ "input": { "username": USERNAME, "password": PASSWORD } });
    let response = hub.post(Some(secret), login, credentials).await;
    assert_eq!(
        error(&response),
        (
            "This token may only read.".to_owned(),
            "FORBIDDEN".to_owned()
        )
    );
    assert_eq!(footprint(&pool).await, before);
    let me = hub.post(Some(secret), ME, json!({})).await;
    assert_eq!(me["data"]["me"]["username"], USERNAME, "it may still read");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_write_key_runs_an_ordinary_mutation(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let key = made(&hub, &token, "writer", "WRITE").await;
    let secret = key["secret"].as_str().unwrap();
    let site = r#"mutation { createSite(name: "office", subnet: "10.40.0.0/24") { id name } }"#;
    let response = hub.post(Some(secret), site, json!({})).await;
    assert_eq!(
        response["data"]["createSite"]["name"], "office",
        "{response}"
    );
    let by: String =
        sqlx::query_scalar("select operator_name from events.audit_log where target_type = 'site'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(by, USERNAME);
}
