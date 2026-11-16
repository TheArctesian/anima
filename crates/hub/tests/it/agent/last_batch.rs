//! What the hub keeps of an agent's last batch. MET-018, R-098 a: the stale watch the hub
//! spawns marks an agent stale once no batch came for more than 30 seconds, with one warning;
//! the next batch clears it with one note. `Device.metricsStale` serves the mark, and a stale
//! agent never makes the device Down. The time of the last batch never moves back. R-118: the
//! version of the newest batch is kept and served as `Device.agentVersion`, only while the
//! device's source is the agent.

use std::time::Duration;

use anima_hub::{
    clock::Clock,
    fleet::{events::Bus, metrics::spawn_stale_watch},
};
use axum::http::StatusCode;
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    batches::cpu_batch,
    harness::{Agents, PVE_01, metrics_events, pushing_now, refusal},
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

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_018_the_time_of_the_last_batch_never_moves_back(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let pushed = agents.push(&token, &cpu_batch(PVE_01, now, 5.0)).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    // A batch whose request started earlier commits later, as one behind a slow lock would.
    agents.clock.advance(TimeDelta::seconds(-20));
    let early = now - TimeDelta::seconds(20);
    let pushed = agents.push(&token, &cpu_batch(PVE_01, early, 5.0)).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    let last: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT last_batch_at FROM metrics.agents WHERE device_id = $1::uuid")
            .bind(PVE_01)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(last, now);
}

/// The device's `agentVersion` as the operator reads it.
async fn agent_version(agents: &Agents) -> Value {
    let query = "query($id: ID!) { device(id: $id) { agentVersion } }";
    let answer: Value = agents.operate(query, json!({ "id": PVE_01 })).await;
    assert!(answer["errors"].is_null(), "{answer}");
    answer["data"]["device"]["agentVersion"].clone()
}

/// A CPU batch of PVE-01 at `at`, signed with `version` when there is one.
fn signed(at: chrono::DateTime<chrono::Utc>, version: Option<&str>) -> Value {
    let mut batch = cpu_batch(PVE_01, at, 5.0);
    if let Some(version) = version {
        batch["agentVersion"] = json!(version);
    }
    batch
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_118_the_version_of_the_newest_batch_is_served_and_an_older_agent_has_none(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    assert_eq!(agent_version(&agents).await, Value::Null, "no batch yet");

    let pushed = agents.push(&token, &signed(now, Some("0.3.1"))).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    assert_eq!(agent_version(&agents).await, json!("0.3.1"));

    agents.clock.advance(TimeDelta::seconds(10));
    let later = now + TimeDelta::seconds(10);
    let pushed = agents.push(&token, &signed(later, Some("0.4.0"))).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    assert_eq!(agent_version(&agents).await, json!("0.4.0"));

    // A batch that started earlier and commits later leaves the newer version.
    agents.clock.advance(TimeDelta::seconds(-20));
    let pushed = agents.push(&token, &signed(now, Some("0.3.1"))).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    assert_eq!(agent_version(&agents).await, json!("0.4.0"));

    // An agent older than R-118 sends no version: the hub no longer claims one.
    agents.clock.advance(TimeDelta::seconds(30));
    let pushed = agents
        .push(&token, &signed(later + TimeDelta::seconds(10), None))
        .await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    assert_eq!(agent_version(&agents).await, Value::Null);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_118_a_new_token_forgets_the_version_and_a_bad_one_stores_nothing(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let pushed = agents.push(&token, &signed(now, Some("0.3.1"))).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);

    agents.clock.advance(TimeDelta::seconds(10));
    let bad = signed(now + TimeDelta::seconds(10), Some("0.3.1 beta"));
    assert_eq!(
        agents.push(&token, &bad).await,
        (StatusCode::BAD_REQUEST, refusal("MALFORMED"))
    );
    assert_eq!(
        agent_version(&agents).await,
        json!("0.3.1"),
        "a refused batch changes nothing"
    );

    let again = agents.token_for(PVE_01).await;
    assert_eq!(
        agent_version(&agents).await,
        Value::Null,
        "a new agent has sent nothing yet"
    );
    let pushed = agents
        .push(&again, &signed(now + TimeDelta::seconds(10), Some("0.4.0")))
        .await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    assert_eq!(agent_version(&agents).await, json!("0.4.0"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_118_a_device_that_leaves_the_agent_or_is_revoked_shows_no_version(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let pushed = agents.push(&token, &signed(now, Some("0.3.1"))).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);

    let to_ssh =
        "mutation($id: ID!) { updateDevice(id: $id, input: { metricsSource: SSH }) { id } }";
    let moved: Value = agents.operate(to_ssh, json!({ "id": PVE_01 })).await;
    assert!(moved["errors"].is_null(), "{moved}");
    assert_eq!(
        agent_version(&agents).await,
        Value::Null,
        "the version of an agent belongs to an AGENT device"
    );

    let again = agents.token_for(PVE_01).await;
    let pushed = agents.push(&again, &signed(now, Some("0.4.0"))).await;
    assert_eq!(pushed.0, StatusCode::NO_CONTENT);
    let revoke = "mutation($id: ID!) { revokeAgentToken(deviceId: $id) { id } }";
    let revoked: Value = agents.operate(revoke, json!({ "id": PVE_01 })).await;
    assert!(revoked["errors"].is_null(), "{revoked}");
    assert_eq!(
        agent_version(&agents).await,
        Value::Null,
        "a revoked agent's version is gone"
    );
}
