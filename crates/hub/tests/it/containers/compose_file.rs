//! `composeFileCheck` and `composeFromDockerRun` through the hub's router (HOST-022, R-150):
//! a signed-in caller and an API key get an answer, a caller who is not signed in gets none,
//! and the text of the caller reaches no log line, no event and no audit entry (LOG-007).

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::fleet::events::process_log;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

const CHECK: &str = "query($file: String!) { composeFileCheck(file: $file) { syntaxValid cut \
    problems { code severity line column service key } } }";

const FROM_RUN: &str = "query($command: String!) { composeFromDockerRun(command: $command) { \
    composeFile service refusal refusedAt notes notConverted } }";

const CREATE_KEY: &str =
    "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }";

/// A value that the operator must not find in a log, an event or a problem.
const SECRET: &str = "s3cr3t-composeValue-41";

/// A Compose file with errors and warnings, whose environment holds [`SECRET`].
fn file_with_secret() -> String {
    format!(
        "version: '3'\nservices:\n  web:\n    environment:\n      PASSWORD: {SECRET}\n    \
         ports: {SECRET}\n    labelz: 1\n"
    )
}

async fn signed_in(pool: &PgPool) -> (Hub, String) {
    set_operator(pool).await;
    let hub = Hub::new(pool, start());
    let token = hub.token().await;
    (hub, token)
}

/// Collects everything the process log writes.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("capture lock")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// How many rows of `table` hold `text` anywhere.
async fn rows_with(pool: &PgPool, table: &str, text: &str) -> i64 {
    let query = format!("select count(*) from {table} row where row::text like $1");
    sqlx::query_scalar(&query)
        .bind(format!("%{text}%"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn host_022_a_signed_in_caller_gets_the_problems_and_never_a_value(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let answer = hub
        .post(Some(&token), CHECK, json!({ "file": file_with_secret() }))
        .await;
    let check = &answer["data"]["composeFileCheck"];
    assert_eq!(check["syntaxValid"], false, "{answer}");
    let codes: Vec<&str> = check["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|problem| problem["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        [
            "VERSION_OBSOLETE",
            "NO_IMAGE_OR_BUILD",
            "WRONG_TYPE",
            "UNKNOWN_SERVICE_KEY"
        ]
    );
    assert!(!answer.to_string().contains(SECRET), "{answer}");

    let good = "services:\n  web:\n    image: nginx:1.27\n";
    let answer = hub.post(Some(&token), CHECK, json!({ "file": good })).await;
    let check = &answer["data"]["composeFileCheck"];
    assert_eq!(check["syntaxValid"], true, "{answer}");
    assert_eq!(check["problems"], json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn host_022_a_command_is_converted_or_refused_with_a_code(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let command = "docker run -d --name web -p 8080:80 -e TZ=UTC nginx:1.27";
    let answer = hub
        .post(Some(&token), FROM_RUN, json!({ "command": command }))
        .await;
    let converted = &answer["data"]["composeFromDockerRun"];
    assert_eq!(converted["service"], "web", "{answer}");
    assert_eq!(converted["refusal"], Value::Null);
    assert_eq!(converted["notes"], json!(["DETACH_NOT_NEEDED"]));
    let file = converted["composeFile"].as_str().unwrap();
    assert!(file.contains("image: \"nginx:1.27\""), "{file}");

    let refused = format!("docker run -e P={SECRET} nginx | tee out");
    let answer = hub
        .post(Some(&token), FROM_RUN, json!({ "command": refused }))
        .await;
    assert_eq!(answer.get("errors"), None, "{answer}");
    let converted = &answer["data"]["composeFromDockerRun"];
    assert_eq!(converted["refusal"], "PIPE");
    assert_eq!(converted["composeFile"], Value::Null);
    assert!(!answer.to_string().contains(SECRET), "{answer}");
}

#[sqlx::test(migrations = "./migrations")]
async fn host_022_a_caller_who_is_not_signed_in_gets_no_answer(pool: PgPool) {
    let (hub, _) = signed_in(&pool).await;
    for (document, variables) in [
        (CHECK, json!({ "file": file_with_secret() })),
        (FROM_RUN, json!({ "command": "docker run nginx" })),
    ] {
        let answer = hub.post(None, document, variables).await;
        assert_eq!(answer["data"], Value::Null, "{answer}");
        assert_eq!(answer["errors"][0]["extensions"]["code"], "UNAUTHENTICATED");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn host_022_a_read_key_uses_both_queries(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let input = json!({ "name": "agent", "scope": "READ", "madeFor": "AGENT",
        "expiresAt": start() + chrono::TimeDelta::days(30) });
    let made = hub
        .post(Some(&token), CREATE_KEY, json!({ "input": input }))
        .await;
    let key = made["data"]["createApiKey"]["secret"].as_str().unwrap();
    let answer = hub
        .post(Some(key), CHECK, json!({ "file": "services: {}\n" }))
        .await;
    let problems = &answer["data"]["composeFileCheck"]["problems"];
    assert_eq!(problems[0]["code"], "SERVICES_EMPTY", "{answer}");
    let answer = hub
        .post(
            Some(key),
            FROM_RUN,
            json!({ "command": "docker run redis:7" }),
        )
        .await;
    assert_eq!(answer["data"]["composeFromDockerRun"]["service"], "redis");
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_the_text_of_the_caller_reaches_no_log_event_or_audit_entry(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let guard = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let (hub, token) = signed_in(&pool).await;
    let command = format!("docker run -e PASSWORD={SECRET} nginx");
    let inline = format!(r#"{{ composeFileCheck(file: "{SECRET}: [") {{ syntaxValid }} }}"#);
    for (document, variables) in [
        (CHECK, json!({ "file": file_with_secret() })),
        (CHECK, json!({ "file": format!("{SECRET}: [") })),
        (
            FROM_RUN,
            json!({ "command": format!("{command} ; {SECRET}") }),
        ),
        // A variable of the wrong type: the hub's error must not quote it.
        (CHECK, json!({ "file": [SECRET] })),
        (inline.as_str(), json!({})),
    ] {
        let answer = hub.post(Some(&token), document, variables).await;
        assert!(!answer.to_string().contains(SECRET), "{answer}");
    }
    // The converted file gives the value back to the caller who sent it, and to no one else.
    let answer = hub
        .post(Some(&token), FROM_RUN, json!({ "command": command }))
        .await;
    let file = answer["data"]["composeFromDockerRun"]["composeFile"].as_str();
    assert!(file.unwrap().contains(SECRET), "{answer}");
    tracing::error!("capture control line");
    drop(guard);

    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("capture control line"),
        "the log was not captured"
    );
    assert!(!text.contains(SECRET), "the text is in the process log");
    for table in ["events.event_log", "events.audit_log"] {
        assert_eq!(rows_with(&pool, table, SECRET).await, 0, "{table}");
    }
}
