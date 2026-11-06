//! The ticket token reaches the client once and nowhere else: not a stored row, an audit
//! entry, an event, an error or the process log (LOG-007, K4 §1).

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::{events::process_log, sessions::Live};
use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    PVE_01,
    harness::schema_at,
    now,
    starting::{end_request, ready, redeem, start_request, started},
};

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

/// Every text column of the sessions table and both logs, as one string.
async fn stored_text(pool: &PgPool) -> String {
    let rows: Vec<String> = sqlx::query_scalar(
        "select row_to_json(s)::text from sessions.sessions s \
         union all select row_to_json(a)::text from events.audit_log a \
         union all select row_to_json(e)::text from events.event_log e",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    rows.join("\n")
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_the_ticket_token_is_never_stored_logged_or_in_an_error(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // While exactly one dispatcher is registered, tracing-core builds a callsite's interest
    // from the registering thread's own default, so another test thread with no subscriber
    // would cache the sessions callsites as disabled for this one too. A second registered
    // dispatcher makes every registration ask all of them.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    tracing::info!("log_007 capture marker");

    let live = Live::default();
    let schema = schema_at(&pool, now(), &live);
    ready(&pool, &[PVE_01]).await;
    let ended = started(&schema, PVE_01).await;
    let swept = started(&schema, PVE_01).await;
    let refused = started(&schema, PVE_01).await;
    let tokens: Vec<String> = [&ended, &swept, &refused]
        .iter()
        .map(|ticket| ticket["token"].as_str().unwrap().to_owned())
        .collect();

    // A refused redemption, an operator end, a sweep, and a refused start afterwards.
    assert!(redeem(&live, &refused, now() + TimeDelta::minutes(1)).is_none());
    let end = schema
        .execute(end_request(ended["sessionId"].as_str().unwrap()))
        .await;
    assert!(end.errors.is_empty(), "{:?}", end.errors);
    let later = schema_at(&pool, now() + TimeDelta::minutes(2), &live);
    started(&later, PVE_01).await;
    let error = later
        .execute(start_request(
            PVE_01,
            &serde_json::json!({ "widthPx": 0, "heightPx": 0 }),
        ))
        .await;
    let error_text = format!(
        "{:?} {}",
        error.errors,
        serde_json::to_string(&error.errors).unwrap()
    );

    let stored = stored_text(&pool).await;
    let logged = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("log_007 capture marker"),
        "the capture works: {logged}"
    );
    for token in &tokens {
        assert!(!stored.contains(token.as_str()), "stored: {stored}");
        assert!(!logged.contains(token.as_str()), "logged: {logged}");
        assert!(!error_text.contains(token.as_str()), "error: {error_text}");
    }
}
