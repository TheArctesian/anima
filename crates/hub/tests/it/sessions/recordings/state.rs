//! `Session.recording` (RA-040, G-5, API-005): `OFF` for a session with no recording row,
//! `STOPPED` for a row with a stop reason, `ON` for any other row, live or ended; one statement
//! for a page of sessions, however many it holds.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anima_hub::sessions::Live;
use serde_json::Value;
use sqlx::PgPool;
use tracing::{Event, Subscriber, subscriber::set_default};
use tracing_subscriber::{Layer, layer::Context, layer::SubscriberExt as _, registry::Registry};

use crate::sessions::{
    DOCKER_HOST, PVE_01, Row,
    harness::{TestSchema, data, schema_recording_to},
    insert_all, now, session_id,
};

/// A recording row of session `n`, ended or not, stopped with `stopped` or not.
async fn record(pool: &PgPool, n: u128, ended: bool, stopped: Option<&str>) {
    sqlx::query(
        "insert into sessions.recordings (id, session_id, device_id, device_name, protocol, \
                format, key_sealed, started_at, ended_at, complete, stopped_reason, expires_at) \
         select gen_random_uuid(), s.id, s.device_id, s.device_name, s.protocol, 'asciicast', \
                '\\x00'::bytea, s.started_at, case when $2 then s.started_at end, $2, $3, \
                s.started_at + interval '30 days' \
         from sessions.sessions s where s.id = $1",
    )
    .bind(session_id(n))
    .bind(ended)
    .bind(stopped)
    .execute(pool)
    .await
    .unwrap();
}

fn schema(pool: &PgPool) -> TestSchema {
    schema_recording_to(pool, now(), &Live::default(), None)
}

/// Each session's id and `recording`, newest first.
async fn states(schema: &TestSchema) -> Vec<(String, String)> {
    let page = data(schema, "{ sessions { edges { node { id recording } } } }").await;
    page["sessions"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            let node = &edge["node"];
            let text = |value: &Value| value.as_str().unwrap().to_owned();
            (text(&node["id"]), text(&node["recording"]))
        })
        .collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_040_a_session_shows_off_on_or_stopped_from_its_recording_row(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::ended(1, PVE_01, 0, "operator"),
            Row::live(2, PVE_01, 1),
            Row::ended(3, DOCKER_HOST, 2, "remote_closed"),
            Row::live(4, DOCKER_HOST, 3),
            Row::ended(5, PVE_01, 4, "operator"),
        ],
    )
    .await;
    record(&pool, 2, false, None).await;
    record(&pool, 3, true, None).await;
    record(&pool, 4, false, Some("disk_full")).await;
    record(&pool, 5, true, Some("size_limit")).await;

    let id = |n| session_id(n).to_string();
    let expected = [
        (5, "STOPPED"),
        (4, "STOPPED"),
        (3, "ON"),
        (2, "ON"),
        (1, "OFF"),
    ]
    .map(|(n, state)| (id(n), state.to_owned()));
    assert_eq!(states(&schema(&pool)).await, expected);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn ra_040_a_recording_that_stops_shows_stopped_in_the_next_request(pool: PgPool) {
    insert_all(&pool, &[Row::live(1, PVE_01, 0)]).await;
    record(&pool, 1, false, None).await;
    let schema = schema(&pool);
    assert_eq!(states(&schema).await[0].1, "ON");

    sqlx::query("update sessions.recordings set stopped_reason = 'write_failed'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(states(&schema).await[0].1, "STOPPED", "no cached state");
}

/// Counts the statements sqlx runs on this thread.
struct Statements(Arc<AtomicUsize>);

impl<S: Subscriber> Layer<S> for Statements {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() == "sqlx::query" {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// The statements `query` runs.
async fn statements(schema: &TestSchema, query: &str) -> usize {
    let count = Arc::new(AtomicUsize::new(0));
    let guard = set_default(Registry::default().with(Statements(count.clone())));
    tracing::callsite::rebuild_interest_cache();
    data(schema, query).await;
    drop(guard);
    count.load(Ordering::SeqCst)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn api_005_session_recording_costs_one_statement_per_page(pool: PgPool) {
    let rows: Vec<Row> = (1..=6).map(|n| Row::live(n, PVE_01, 0)).collect();
    insert_all(&pool, &rows).await;
    for n in [2, 4] {
        record(&pool, n, false, None).await;
    }
    let schema = schema(&pool);

    let without = "{ sessions { edges { node { id } } } }";
    // Discarded: while other tests register tracing call sites, the first count after a new
    // subscriber can miss statements; the next is exact.
    statements(&schema, without).await;
    let without = statements(&schema, without).await;
    let with = statements(&schema, "{ sessions { edges { node { id recording } } } }").await;

    assert!(without > 0, "the statements were counted");
    assert_eq!(with, without + 1, "one statement for the page's recordings");
}
