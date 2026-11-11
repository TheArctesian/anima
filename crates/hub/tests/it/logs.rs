//! Hub faults reach the process log by kind and cause chain, never with SQL or a token (K7,
//! LOG-007, API-010, H-5).

use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use anima_hub::events::process_log;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::hub::{Hub, PASSWORD, USERNAME, set_operator, start};

/// Collects everything the process log writes.
#[derive(Clone, Default)]
pub struct Capture(Arc<Mutex<Vec<u8>>>);

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

impl Capture {
    pub fn text(&self) -> String {
        String::from_utf8(self.0.lock().expect("capture lock").clone()).unwrap()
    }
}

#[tokio::test]
async fn log_002_the_hub_router_logs_each_request_by_its_route() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("info", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let hub = unreachable_hub();

    let health = Request::get("/health").body(Body::empty()).unwrap();
    assert_eq!(hub.send(health).await.0, StatusCode::OK);
    hub.post(None, "{ __typename }", json!({})).await;

    let served: Vec<serde_json::Value> = capture
        .text()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|line| line["fields"]["message"] == "request served")
        .collect();
    let paths: Vec<_> = served.iter().map(|line| &line["span"]["path"]).collect();
    assert_eq!(paths, [&json!("/health"), &json!("/graphql")], "{served:?}");
}

/// A hub whose database cannot be reached: each query fails with a pool or I/O error.
fn unreachable_hub() -> Hub {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(300))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    Hub::new(&pool, start())
}

#[tokio::test]
async fn api_010_h_5_an_unreachable_database_is_logged_by_kind_at_both_edges() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("error", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    let hub = unreachable_hub();

    let login = hub.login(USERNAME, PASSWORD).await;
    assert_eq!(
        login["errors"][0]["extensions"]["code"], "UNAVAILABLE",
        "{login}"
    );
    // The source is a pool timeout or a connection error, logged by kind.
    let resolver = r#""message":"request failed: database_unreachable: database "#;
    assert_eq!(
        capture.text().matches(resolver).count(),
        1,
        "through the resolver, at the API boundary: {}",
        capture.text()
    );
    assert!(capture.text().contains(r#""code":"UNAVAILABLE""#));

    let token = URL_SAFE_NO_PAD.encode([9_u8; 32]);
    let me = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(me["errors"][0]["extensions"]["code"], "UNAVAILABLE", "{me}");
    let text = capture.text();
    assert_eq!(
        text.matches(resolver).count(),
        2,
        "and through identify, at the HTTP edge: {text}"
    );
    assert_eq!(text.matches("database_unreachable").count(), 2, "{text}");
    for leak in [
        token.as_str(),
        PASSWORD,
        "select",
        "insert",
        "access.",
        "127.0.0.1",
    ] {
        assert!(!text.contains(leak), "{leak:?} in {text}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn h_5_a_failed_token_lookup_at_the_edge_is_logged_with_its_sqlstate(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    sqlx::query("alter table access.api_tokens rename to api_tokens_gone")
        .execute(&pool)
        .await
        .unwrap();
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("error", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let response = hub.post(Some(&token), "{ __typename }", json!({})).await;

    assert_eq!(
        response,
        json!({
            "data": null,
            "errors": [{ "message": "Something went wrong on the hub.",
                         "locations": [{ "line": 0, "column": 0 }],
                         "extensions": { "code": "INTERNAL" } }]
        })
    );
    let text = capture.text();
    assert!(
        text.contains("request failed: database: database error 42P01"),
        "{text}"
    );
    assert!(text.contains(r#""code":"INTERNAL""#), "{text}");
    for leak in [token.as_str(), "api_tokens", "select"] {
        assert!(!text.contains(leak), "{leak:?} in {text}");
    }
}

#[tokio::test]
async fn log_007_a_malformed_login_input_keeps_its_password_out_of_the_log() {
    const SECRET: &str = "hunter2-correct-horse";
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let hub = unreachable_hub();
    let query = "mutation($input: LoginInput!) { login(input: $input) { token } }";

    let wrong_shape = hub.post(None, query, json!({ "input": SECRET })).await;
    assert_eq!(
        wrong_shape["errors"][0]["extensions"]["code"], "VALIDATION",
        "{wrong_shape}"
    );
    assert!(!wrong_shape.to_string().contains(SECRET), "{wrong_shape}");
    for input in [
        json!({ "username": 5, "password": SECRET }),
        json!({ "username": USERNAME, "password": SECRET, "extra": SECRET }),
        json!({ "username": USERNAME, "password": SECRET, "totpCode": 7 }),
    ] {
        let response = hub.post(None, query, json!({ "input": input })).await;
        assert!(response["errors"][0].is_object(), "{response}");
    }

    let text = capture.text();
    assert!(text.contains("argument refused"), "{text}");
    assert!(!text.contains(SECRET), "{text}");
}
