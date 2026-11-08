//! A channel whose stored settings this version cannot read: the list leaves it out and logs
//! only its id, and the operator can still delete it (NOT-002, LOG-005, LOG-007).

use std::{
    future::Future,
    io,
    sync::{Arc, Mutex},
};

use anima_hub::events::process_log::subscriber;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{DELETE, channels, create, data, schema_for, webhook};

/// Settings no kind reads: a database writer or a later version put them there.
const UNREADABLE: &str = r#"{"not-a-setting": "settings-s3cr3t"}"#;

async fn break_settings(pool: &PgPool, id: &Value) {
    let id: uuid::Uuid = id.as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE notifications.channels SET settings = $2::jsonb WHERE id = $1")
        .bind(id)
        .bind(UNREADABLE)
        .execute(pool)
        .await
        .unwrap();
}

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

/// The process log this thread writes while `run` runs, and its result.
async fn logged<R>(run: impl Future<Output = R>) -> (String, R) {
    let capture = Capture::default();
    let writer = capture.clone();
    let guard =
        tracing::subscriber::set_default(subscriber("info", move || writer.clone()).unwrap());
    tracing::callsite::rebuild_interest_cache();
    let result = run.await;
    tracing::error!("capture control line");
    drop(guard);
    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("capture control line"),
        "the log was not captured"
    );
    (text, result)
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_a_channel_whose_settings_cannot_be_read_is_left_out_of_the_list(pool: PgPool) {
    let schema = schema_for(&pool);
    let broken = create(&schema, webhook("Broken")).await;
    let kept = create(&schema, webhook("Kept")).await;
    break_settings(&pool, &broken["id"]).await;

    let (log, listed) = logged(channels(&schema)).await;
    let ids: Vec<&Value> = listed.iter().map(|channel| &channel["id"]).collect();
    assert_eq!(ids, [&kept["id"]]);
    assert!(
        log.contains(broken["id"].as_str().unwrap()),
        "the log names no channel"
    );
    assert!(
        !log.contains("settings-s3cr3t"),
        "the log quotes the stored settings"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_channel_whose_settings_cannot_be_read_is_deleted_with_one_entry(pool: PgPool) {
    let schema = schema_for(&pool);
    let broken = create(&schema, webhook("Broken")).await;
    break_settings(&pool, &broken["id"]).await;

    let deleted = data(&schema, DELETE, json!({ "id": broken["id"] })).await;
    assert_eq!(deleted["deleteNotificationChannel"], broken["id"]);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications.channels")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
    let changes: Vec<Value> = sqlx::query_scalar(
        "SELECT change FROM events.audit_log WHERE action = 'delete' AND target_id = $1",
    )
    .bind(broken["id"].as_str().unwrap())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(changes, [json!({ "name": "Broken", "kind": "webhook" })]);
}
