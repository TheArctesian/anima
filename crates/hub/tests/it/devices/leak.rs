//! What the credential mutations let out (LOG-007, API-010). In a query that parses, a
//! credential of the wrong shape or type is refused without quoting it, in the response or in
//! the process log. A query that does not parse is echoed to its sender by the GraphQL parser
//! (pinned below, handed off under H-6), never to the log. Missing wiring shows only `INTERNAL`.

use std::{
    future::Future,
    io,
    sync::{Arc, Mutex},
};

use anima_hub::{
    clock::{Clock, FixedClock},
    devices::graphql::{Mutation, Query},
    events::{Bus, process_log::subscriber},
};
use async_graphql::{EmptySubscription, Request, Schema, Variables};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    credential::{PVE_01, SET},
    harness::{error_json, hardened_schema, now, signed_in},
};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Runs `request` with this thread's log routed to a buffer; returns the log text and the
/// result. A control line written after the request proves the buffer receives this thread's
/// log, so an empty log is not a capture that silently failed.
pub async fn captured<F, R>(request: impl FnOnce() -> F) -> (String, R)
where
    F: Future<Output = R>,
{
    let capture = Capture::default();
    let writer = capture.clone();
    let log = subscriber("trace", move || writer.clone()).unwrap();
    let guard = tracing::subscriber::set_default(log);
    tracing::callsite::rebuild_interest_cache();
    let result = request().await;
    tracing::error!("capture control line");
    drop(guard);
    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("capture control line"),
        "the log was not captured"
    );
    (text, result)
}

/// Fails if `secret` appears in any of `places`, without printing the place (it would print
/// the secret).
pub fn assert_absent(secret: &str, places: &[&str]) {
    for (index, place) in places.iter().enumerate() {
        assert!(!place.contains(secret), "a secret appears in place {index}");
    }
}

const INLINE_PASSWORD: &str = "hunter2-inline-bare-string";
const INLINE_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----inlinekeybody";
const VARIABLE_PASSWORD: &str = "hunter2-variable-bare-string";
const VARIABLE_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----variablekeybody";

fn inline(credential: &str) -> String {
    format!(
        r#"mutation {{ setSshCredential(deviceId: "{PVE_01}", credential: {credential}) {{ credentialStored }} }}"#
    )
}

fn variables(credential: &Value) -> Variables {
    Variables::from_json(json!({ "id": PVE_01, "credential": credential }))
}

/// Each malformed request, with the secret it carries and whether an operator sent it.
fn malformed() -> Vec<(Request, &'static str)> {
    vec![
        (
            signed_in(&inline(&format!("{INLINE_PASSWORD:?}"))),
            INLINE_PASSWORD,
        ),
        (
            signed_in(&inline(&format!("{{ privateKey: {INLINE_KEY:?} }}"))),
            INLINE_KEY,
        ),
        (
            signed_in(SET).variables(variables(&json!(VARIABLE_PASSWORD))),
            VARIABLE_PASSWORD,
        ),
        (
            signed_in(SET).variables(variables(&json!({ "privateKey": VARIABLE_KEY }))),
            VARIABLE_KEY,
        ),
        // Arguments are parsed before the guard runs, so a caller without an operator reaches
        // the parser too.
        (
            Request::new(SET).variables(variables(&json!(VARIABLE_PASSWORD))),
            VARIABLE_PASSWORD,
        ),
    ]
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_a_credential_that_does_not_parse_is_refused_without_quoting_it(pool: PgPool) {
    let schema = hardened_schema(&pool);
    for (index, (request, secret)) in malformed().into_iter().enumerate() {
        let (log, response) = captured(|| schema.execute(request)).await;
        let error = error_json(&response);
        let body = secret.trim_start_matches("-----BEGIN OPENSSH PRIVATE KEY-----");
        let debug = format!("{:?}", response.errors);
        let message = response.errors[0].message.clone();
        assert_absent(body, &[&error.to_string(), &debug, &message, &log]);
        assert_eq!(error["extensions"]["code"], "VALIDATION", "request {index}");
        assert_eq!(
            error["extensions"]["fields"][0]["field"], "credential",
            "request {index}"
        );
    }
    assert_eq!(secret_rows(&pool).await, 0);
}

async fn secret_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM credentials.secrets")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_010_without_the_vault_or_the_credential_loader_the_operator_gets_internal(
    pool: PgPool,
) {
    // Everything the mutation and the field need except the vault and the loader.
    let (bus, clock) = (Bus::new(4), Arc::new(FixedClock(now())) as Arc<dyn Clock>);
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(pool.clone())
        .data(bus)
        .data(clock)
        .finish();
    let set = signed_in(SET).variables(variables(&json!({ "password": "a-password" })));
    let read = signed_in(&format!(
        r#"{{ device(id: "{PVE_01}") {{ ssh {{ credentialStored }} }} }}"#
    ));
    for request in [set, read] {
        let error = error_json(&schema.execute(request).await);
        assert_eq!(error["extensions"]["code"], "INTERNAL", "{error}");
        let text = error.to_string();
        for leak in ["Vault", "DataLoader", "CredentialStored", "Data `"] {
            assert!(!text.contains(leak), "{text}");
        }
    }
    assert_eq!(secret_rows(&pool).await, 0);
}

/// Inline credentials that GraphQL validation refuses before the parser sees them, each with
/// the secret it carries. Validation errors name types and fields, never the values.
const REFUSED_SHAPES: [(&str, &str); 7] = [
    (
        r#"{ passwrd: "shape-misspelt-secret" }"#,
        "shape-misspelt-secret",
    ),
    (
        r#"{ password: "shape-both-secret", privateKey: { key: "-----BEGIN OPENSSH PRIVATE KEY-----shapebothkey" } }"#,
        "shape-both-secret",
    ),
    (
        r#"{ password: "shape-both-other", privateKey: { key: "-----BEGIN OPENSSH PRIVATE KEY-----shapebothkey" } }"#,
        "shapebothkey",
    ),
    ("{ password: 918273645 }", "918273645"),
    (
        r#"{ password: "shape-extra-secret", extra: "x" }"#,
        "shape-extra-secret",
    ),
    (r#"["shape-list-secret"]"#, "shape-list-secret"),
    ("{ password: null }", "null-password-marker"),
];

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_inline_credentials_of_the_wrong_shape_are_refused_without_quoting_them(
    pool: PgPool,
) {
    let schema = hardened_schema(&pool);
    for (index, (credential, secret)) in REFUSED_SHAPES.into_iter().enumerate() {
        let request = signed_in(&inline(credential));
        let (log, response) = captured(|| schema.execute(request)).await;
        assert!(!response.errors.is_empty(), "shape {index} was accepted");
        let body = serde_json::to_string(&response).unwrap();
        assert_absent(secret, &[&body, &format!("{:?}", response.errors), &log]);
    }
    assert_eq!(secret_rows(&pool).await, 0);
}

/// A query that does not parse. The async-graphql parser quotes the text around the error, so
/// a secret written inline next to a syntax error comes back in the response to its sender.
/// That is known and handed to WP-05 under H-6 (refuse unauthenticated requests before parsing);
/// this test pins it, and pins that the process log does not get it.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_a_query_with_a_syntax_error_echoes_its_text_to_the_sender_but_not_to_the_log(
    pool: PgPool,
) {
    let secret = "syntax-error-secret";
    let query = format!(
        r#"mutation {{ setSshCredential(deviceId: "{PVE_01}", credential: {{ password: "{secret}" ) }}"#
    );
    let schema = hardened_schema(&pool);
    let (log, response) = captured(|| schema.execute(signed_in(&query))).await;
    assert_eq!(response.errors.len(), 1);
    let body = serde_json::to_string(&response).unwrap();
    assert!(
        body.contains(secret),
        "the parser no longer echoes the query; update H-6"
    );
    assert_absent(secret, &[&log]);
    assert_eq!(secret_rows(&pool).await, 0);
}
