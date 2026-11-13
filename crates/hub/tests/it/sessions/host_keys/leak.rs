//! What `testConnection` lets out (LOG-007, API-010, WP-06 H-4): an input that does not parse
//! is refused without quoting its credential, in the response or in the process log, and a
//! test that signs in logs no secret.

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        events::process_log::subscriber,
        sessions::graphql::{Query, Subscription},
    },
    schema::harden,
};
use async_graphql::{Request, Response, Schema, Variables};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    TEST, accept_directly, loopback_live, password, point_at,
    server::{PASSWORD, SshServer, ed25519},
    store_credential,
};
use crate::sessions::{
    PVE_01,
    harness::{Mutation, TestSchema, error_json, signed_in_until, vault},
    now,
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

/// Runs `request` with this thread's log in a buffer; a control line proves the capture.
async fn logged(schema: &TestSchema, request: Request) -> (String, Response) {
    let capture = Capture::default();
    let writer = capture.clone();
    let log = subscriber("trace", move || writer.clone()).unwrap();
    let guard = tracing::subscriber::set_default(log);
    tracing::callsite::rebuild_interest_cache();
    let response = schema.execute(request).await;
    tracing::error!("capture control line");
    drop(guard);
    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("capture control line"),
        "the log was not captured"
    );
    (text, response)
}

/// The sessions schema, with the process-log extensions `schema.rs` adds when `hardened` (an
/// error without a code then reaches the log, and argument errors are masked in the response).
/// The plain schema shows what the input type itself lets out.
fn schema_with(pool: &PgPool, hardened: bool) -> TestSchema {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let builder = Schema::build(Query, Mutation::default(), Subscription)
        .data(pool.clone())
        .data(loopback_live())
        .data(vault())
        .data(clock);
    if hardened {
        harden(builder, false).finish()
    } else {
        builder.finish()
    }
}

fn hardened(pool: &PgPool) -> TestSchema {
    schema_with(pool, true)
}

const INLINE: &str = "hunter2-inline-secret";
const VARIABLE: &str = "hunter2-variable-secret";

fn inline(input: &str) -> Request {
    signed_in_until(
        &format!("mutation {{ testConnection(input: {input}) {{ reachable }} }}"),
        24,
    )
}

fn with_input(request: Request, input: &Value) -> Request {
    request.variables(Variables::from_json(json!({ "input": input })))
}

fn signed_in(input: &Value) -> Request {
    with_input(signed_in_until(TEST, 24), input)
}

/// Who refuses a malformed input: the hub's parser, with `VALIDATION`, or async-graphql's
/// static validation before any parse, with no code (as for `setSshCredential`; H-6). Both
/// name fields only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Parse,
    Static,
}

/// Each malformed request, the secret it carries and who refuses it.
fn malformed() -> Vec<(Request, &'static str, Refusal)> {
    let device = format!("{PVE_01:?}");
    let both = format!(
        "{{ deviceId: {device}, credential: {{ password: {INLINE:?}, privateKey: {{ key: \"k\" }} }} }}"
    );
    let partial_ssh = json!({ "host": "10.0.0.1", "ssh": { "port": 22 },
        "credential": { "password": VARIABLE } });
    vec![
        (
            inline(&format!("{{ deviceId: {device}, credential: {INLINE:?} }}")),
            INLINE,
            Refusal::Parse,
        ),
        (inline(&both), INLINE, Refusal::Static),
        (signed_in(&json!(VARIABLE)), VARIABLE, Refusal::Parse),
        (
            signed_in(&json!([{ "deviceId": PVE_01, "credential": { "password": VARIABLE } }])),
            VARIABLE,
            Refusal::Parse,
        ),
        (
            signed_in(&json!({ "deviceId": PVE_01, "credential": VARIABLE })),
            VARIABLE,
            Refusal::Parse,
        ),
        (
            signed_in(&json!({ "deviceId": 7.5, "credential": { "password": VARIABLE } })),
            VARIABLE,
            Refusal::Static,
        ),
        (signed_in(&partial_ssh), VARIABLE, Refusal::Static),
        // Arguments are parsed before the guard runs, so a caller without an operator reaches
        // the parser too.
        (
            with_input(
                Request::new(TEST),
                &json!({ "deviceId": PVE_01, "credential": VARIABLE }),
            ),
            VARIABLE,
            Refusal::Parse,
        ),
    ]
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn log_007_a_connection_test_input_that_does_not_parse_never_quotes_its_credential(
    pool: PgPool,
) {
    for hardened in [true, false] {
        refused_without_quoting(&schema_with(&pool, hardened), hardened).await;
    }
}

/// Behind the API boundary every argument refusal is one fixed VALIDATION text (H-6(f)), so
/// which step refused it is told only on the bare schema.
async fn refused_without_quoting(schema: &TestSchema, hardened: bool) {
    for (index, (request, secret, refusal)) in malformed().into_iter().enumerate() {
        let (log, response) = logged(schema, request).await;
        let error = error_json(&response);
        let places = [
            error.to_string(),
            format!("{:?}", response.errors),
            response.errors[0].message.clone(),
            log,
        ];
        for (place, text) in places.iter().enumerate() {
            assert!(!text.contains(secret), "request {index}: place {place}");
        }
        let message = error["message"].as_str().unwrap();
        if hardened {
            assert_eq!(error["extensions"]["code"], "VALIDATION", "request {index}");
            continue;
        }
        let refused_by = if error["extensions"]["code"] == "VALIDATION" {
            Refusal::Parse
        } else {
            assert!(
                message.starts_with("Invalid value for argument \"input"),
                "{message}"
            );
            Refusal::Static
        };
        assert_eq!(refused_by, refusal, "request {index}: {message}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn log_007_a_credential_that_fails_its_checks_is_named_not_quoted(pool: PgPool) {
    let schema = hardened(&pool);
    let oversized = VARIABLE.repeat(100);
    for secret in ["   ", oversized.as_str()] {
        let input = json!({ "deviceId": PVE_01, "credential": { "password": secret } });
        let (log, response) = logged(&schema, signed_in(&input)).await;
        let error = error_json(&response);
        assert_eq!(error["extensions"]["code"], "VALIDATION");
        assert_eq!(
            error["extensions"]["fields"][0]["field"],
            "credential.password"
        );
        assert!(!error.to_string().contains(VARIABLE));
        assert!(!log.contains(VARIABLE));
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn log_007_a_test_that_signs_in_logs_no_secret_and_no_remote_text(pool: PgPool) {
    let host_key = ed25519(1);
    let server = SshServer::start(vec![host_key.clone()], None).await;
    point_at(&pool, &server).await;
    accept_directly(&pool, &host_key).await;
    store_credential(&pool, password("a-stored-password")).await;
    let schema = hardened(&pool);

    let given = json!({ "deviceId": PVE_01, "credential": { "password": PASSWORD } });
    let (log, response) = logged(&schema, signed_in(&given)).await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let result = response.data.into_json().unwrap();
    assert_eq!(result["testConnection"]["reachable"], true);
    assert!(!log.contains(PASSWORD));
    assert!(!log.contains("russh"), "no server id reaches the log");
}
