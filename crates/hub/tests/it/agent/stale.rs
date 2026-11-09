//! MET-018, R-098 a: the stale watch the hub spawns marks an agent stale once no batch came for
//! more than 30 seconds, with one warning; the next batch clears it with one note.
//! `Device.metricsStale` serves the mark, and a stale agent never makes the device Down.

use std::time::Duration;

use anima_hub::{clock::Clock, events::Bus, metrics::spawn_stale_watch};
use axum::http::StatusCode;
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    batches::cpu_batch,
    harness::{Agents, PVE_01, metrics_events, pushing_now},
};

const STALE: &str = "The agent on pve-01 sent no metrics for 30 seconds; they are stale.";
const FRESH: &str = "The agent on pve-01 sends metrics again.";

/// The device's `metricsStale` and `status` as the operator reads them.
async fn standing(agents: &Agents) -> (bool, String) {
    let query = "query($id: ID!) { device(id: $id) { metricsStale status } }";
    let answer: Value = agents.operate(query, json!({ "id": PVE_01 })).await;
    let device = &answer["data"]["device"];
    let stale = device["metricsStale"]
        .as_bool()
        .unwrap_or_else(|| panic!("{answer}"));
    (stale, device["status"].as_str().unwrap().to_owned())
}

/// Waits up to 10 seconds for the metrics events of `severity` to number `count`.
async fn events_reach(pool: &PgPool, severity: &str, count: usize) -> Vec<String> {
    for _ in 0..100 {
        let written = metrics_events(pool, severity).await;
        if written.len() >= count {
            return written;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    metrics_events(pool, severity).await
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_018_the_spawned_watch_marks_a_silent_agent_stale_and_a_batch_clears_it(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let (unpushed, status) = standing(&agents).await;
    assert!(unpushed, "an agent that never pushed has no fresh metrics");
    assert_eq!(
        agents.push(&token, &cpu_batch(PVE_01, now, 5.0)).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(standing(&agents).await, (false, status.clone()));

    agents.clock.advance(TimeDelta::seconds(31));
    let clock: std::sync::Arc<dyn Clock> = agents.clock.clone();
    let watch = spawn_stale_watch(pool.clone(), Bus::new(16), clock);
    let written = events_reach(&pool, "warning", 1).await;
    assert_eq!(written, [STALE]);
    assert_eq!(standing(&agents).await, (true, status.clone()));
    assert_ne!(status, "DOWN", "a stale agent is never a Down check");
    // Later rounds write nothing more while the agent stays silent.
    tokio::time::sleep(Duration::from_secs(6)).await;
    assert_eq!(metrics_events(&pool, "warning").await, [STALE]);

    let back = now + TimeDelta::seconds(30);
    assert_eq!(
        agents.push(&token, &cpu_batch(PVE_01, back, 6.0)).await.0,
        StatusCode::NO_CONTENT
    );
    let notes = metrics_events(&pool, "info").await;
    assert_eq!(
        notes.iter().filter(|note| *note == FRESH).count(),
        1,
        "{notes:?}"
    );
    assert_eq!(standing(&agents).await, (false, status));
    watch.abort();
}
