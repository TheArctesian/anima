//! WP-10 handoff 4: nothing in the hub's router writes a request's URI or query to the
//! process log, so a token put in a URL never reaches it (LOG-007).

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::events::process_log;
use axum::{
    body::Body,
    http::{Request, header},
};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start};

const MARKER: &str = "SECRET-MARKER";

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

/// A request to `/graphql?token=MARKER` with `method`, `content_type`, `bearer` and `body`.
fn marked(method: &str, content_type: &str, bearer: Option<&str>, body: &str) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/graphql?token={MARKER}&query={MARKER}"))
        .header(header::CONTENT_TYPE, content_type);
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    request.body(Body::from(body.to_owned())).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_a_token_in_the_url_never_reaches_the_process_log(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in sessions::leak: a second registered dispatcher makes each callsite ask every
    // dispatcher, so another test thread cannot cache this test's callsites as disabled.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    tracing::info!("log_007 capture marker");

    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let json = "application/json";
    let requests = [
        // Signed in, and failing in a resolver.
        marked(
            "POST",
            json,
            Some(&token),
            r#"{"query":"{ device(id: \"x\") { id } }"}"#,
        ),
        // Signed in and answered.
        marked(
            "POST",
            json,
            Some(&token),
            r#"{"query":"{ sites { name } }"}"#,
        ),
        // Refused: no token, a bad token, not JSON, not GraphQL, a GET that is no upgrade.
        marked("POST", json, None, r#"{"query":"{ sites { name } }"}"#),
        marked(
            "POST",
            json,
            Some(MARKER),
            r#"{"query":"{ sites { name } }"}"#,
        ),
        marked("POST", "text/plain", None, "{}"),
        marked("POST", json, None, "not json"),
        marked("GET", json, None, ""),
    ];
    let mut statuses = Vec::new();
    for request in requests {
        statuses.push(hub.send(request).await.0.as_u16());
    }

    let logged = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert_eq!(statuses, [200, 200, 200, 200, 415, 400, 405]);
    assert!(
        logged.contains("log_007 capture marker"),
        "the capture works: {logged}"
    );
    assert!(!logged.contains(MARKER), "logged: {logged}");
}
