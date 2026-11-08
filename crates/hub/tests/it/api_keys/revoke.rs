//! `revokeApiKey`: the row goes, the key stops at once on HTTP and at the next re-check on a
//! socket; create and revoke are audited; the secret never reaches a log (D-23, LOG-005,
//! LOG-007).

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::{
    clock::{Clock, FixedClock},
    events::process_log,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{ME, REVOKE, body, coded, count, instant, made, secret, signed_in};
use crate::{
    browser::socket::{FORBIDDEN, ack, init, open, receive, serve_watched},
    hub::{start, unauthenticated},
};

#[sqlx::test(migrations = "./migrations")]
async fn d_23_revoke_deletes_the_key_and_it_stops_at_once(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let key = made(&hub, &token, "deploy", "WRITE").await;
    let secret = key["secret"].as_str().unwrap();
    let id = key["key"]["id"].clone();
    let me = hub.post(Some(secret), ME, json!({})).await;
    assert_eq!(me["data"]["me"]["username"], "operator");

    let revoked = hub.post(Some(&token), REVOKE, json!({ "id": id })).await;
    assert_eq!(revoked, json!({ "data": { "revokeApiKey": id } }));
    assert_eq!(
        count(&pool, "access.api_tokens where kind = 'api_key'").await,
        0
    );
    let after = hub.post(Some(secret), ME, json!({})).await;
    assert_eq!(crate::hub::error(&after), unauthenticated());

    let again = hub.post(Some(&token), REVOKE, json!({ "id": id })).await;
    assert_eq!(coded(&again).0, "NOT_FOUND", "{again}");
    let junk = hub.post(Some(&token), REVOKE, json!({ "id": "x" })).await;
    assert_eq!(
        coded(&junk),
        (
            "VALIDATION".to_owned(),
            json!([{ "field": "id", "message": "That id is not valid." }])
        )
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_revoke_deletes_only_api_keys(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let login_id: uuid::Uuid =
        sqlx::query_scalar("select id from access.api_tokens where kind = 'login'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let response = hub
        .post(Some(&token), REVOKE, json!({ "id": login_id.to_string() }))
        .await;
    assert_eq!(coded(&response).0, "NOT_FOUND", "{response}");
    let me = hub.post(Some(&token), ME, json!({})).await;
    assert_eq!(
        me["data"]["me"]["username"], "operator",
        "the login token stays"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_socket_signed_in_with_a_revoked_key_closes_at_the_recheck(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let key = made(&hub, &token, "watcher", "READ").await;
    let secret = key["secret"].as_str().unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let served = serve_watched(&pool, clock).await;
    let mut socket = open(served.addr, &[]).await;
    let payload = json!({ "authorization": format!("Bearer {secret}") });
    assert_eq!(init(&mut socket, payload).await, Ok(ack()));

    let revoked = hub
        .post(Some(&token), REVOKE, json!({ "id": key["key"]["id"] }))
        .await;
    assert!(revoked["data"]["revokeApiKey"].is_string(), "{revoked}");

    assert_eq!(receive(&mut socket).await, FORBIDDEN);
}

/// The audit rows on API keys, oldest first, as JSON.
async fn key_audits(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar(
        "select to_jsonb(a) from events.audit_log a where target_type = 'api_key' order by at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_create_and_revoke_each_leave_an_audit_entry_and_an_event(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let key = made(&hub, &token, "backup", "READ").await;
    let id = key["key"]["id"].clone();
    hub.post(Some(&token), REVOKE, json!({ "id": id })).await;

    let mut audits = key_audits(&pool).await;
    let summary: Vec<_> = audits
        .iter()
        .map(|row| (row["action"].clone(), row["target_id"].clone()))
        .collect();
    assert_eq!(
        summary,
        [(json!("create"), id.clone()), (json!("delete"), id)]
    );
    // R-033: compare instants, not text.
    let expires_at = audits[0]["change"]["expiresAt"].take();
    assert_eq!(instant(&expires_at), instant(&key["key"]["expiresAt"]));
    assert_eq!(
        audits[0]["change"],
        json!({ "name": "backup", "scope": "read", "expiresAt": null })
    );
    assert_eq!(audits[1]["change"], json!({ "name": "backup" }));
    for row in &audits {
        assert_eq!(row["operator_name"], "operator");
    }
    let events: Vec<String> = sqlx::query_scalar(
        "select message from events.event_log where source = 'access' and message like 'API key%' \
         order by at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        events,
        ["API key backup created.", "API key backup revoked."]
    );
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

#[sqlx::test(migrations = "./migrations")]
async fn log_007_an_api_key_secret_never_reaches_the_process_log(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let guard = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let (hub, token) = signed_in(&pool).await;
    let secret = secret(&hub, &token, "logged", "READ").await;
    hub.post(Some(&secret), ME, json!({})).await;
    hub.post(Some(&secret), "mutation { createApiKey }", json!({}))
        .await;
    hub.post(Some(&secret), "{ nonsense }", json!({})).await;
    let listed = hub
        .post(Some(&token), "{ apiKeys { id } }", json!({}))
        .await;
    let id = listed["data"]["apiKeys"][0]["id"].clone();
    hub.post(Some(&token), REVOKE, json!({ "id": id })).await;
    hub.post(Some(&secret), ME, json!({})).await;
    // A line written after the requests proves this thread's log reached the buffer, so an
    // empty log is not a capture that silently failed.
    tracing::error!("capture control line");
    drop(guard);

    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("capture control line"),
        "the log was not captured"
    );
    // The log is not printed: it would hold the secret.
    assert!(
        !text.contains(body(&secret)),
        "the secret is in the process log"
    );
}
