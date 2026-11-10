//! R-098 e, CR-20: a device creates at most 1024 series in any hour, a batch's new ones
//! included, counted by when the hub created them. Every series one agent can send at once
//! fits; a batch that would pass the cap is refused `MALFORMED` with an event and stores
//! nothing; back-dated points do not reset the count, and an hour later there is room again.

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    batches::{batch, point, samples_of, series_of, warnings},
    harness::{Agents, PERMITTED, PVE_01, pushing_now},
};

/// Every series one agent can send at once: each plain kind, 32 labels of each disk and
/// network kind, and each GPU kind of the 64 GPU indexes (519).
fn every_series() -> Vec<Value> {
    let plain = [
        "cpu_percent",
        "memory_percent",
        "memory_bytes",
        "uptime_seconds",
        "load_1",
        "load_5",
        "load_15",
    ];
    let labelled = [
        "disk_percent",
        "disk_bytes",
        "network_in_bps",
        "network_out_bps",
    ];
    let gpu = [
        "gpu_percent",
        "vram_percent",
        "vram_bytes",
        "gpu_temperature_c",
        "gpu_power_w",
        "gpu_memory_bytes",
    ];
    let mut series: Vec<Value> = plain.iter().map(|kind| json!({ "kind": kind })).collect();
    for kind in labelled {
        series.extend((0..32).map(|label| json!({ "kind": kind, "label": format!("/l{label}") })));
    }
    for kind in gpu {
        series.extend((0..64).map(|index| json!({ "kind": kind, "gpu": index })));
    }
    series
}

/// `count` disk series named from `first` on.
fn disks(first: usize, count: usize) -> Vec<Value> {
    (first..first + count)
        .map(|label| json!({ "kind": "disk_bytes", "label": format!("/extra{label}") }))
        .collect()
}

/// Pushes one point at `at` for each of `series`, 256 series a batch; the status of each.
async fn push_all(
    agents: &Agents,
    token: &str,
    series: &[Value],
    at: DateTime<Utc>,
) -> Vec<StatusCode> {
    let mut statuses = Vec::new();
    for chunk in series.chunks(256) {
        let means = vec![1.0; chunk.len()];
        let pushed = batch(PVE_01, json!(chunk), vec![point(at, json!(means))]);
        statuses.push(agents.push(token, &pushed).await.0);
    }
    statuses
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_098_e_a_device_creates_at_most_1024_series_in_an_hour(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let every = every_series();
    assert_eq!(every.len(), 519);
    let accepted = push_all(&agents, &token, &every, now).await;
    assert!(
        accepted
            .iter()
            .all(|status| *status == StatusCode::NO_CONTENT),
        "{accepted:?}"
    );
    assert_eq!(samples_of(&pool, PVE_01).await, 519);

    let filled = push_all(&agents, &token, &disks(0, 1024 - 519), now).await;
    assert!(
        filled
            .iter()
            .all(|status| *status == StatusCode::NO_CONTENT),
        "{filled:?}"
    );
    assert_eq!(samples_of(&pool, PVE_01).await, 1024);
    // A known series still takes points; one more is refused, and nothing of its batch stays.
    let known = &every[..1];
    let later = now - TimeDelta::seconds(10);
    assert_eq!(
        push_all(&agents, &token, known, later).await,
        [StatusCode::NO_CONTENT]
    );
    let mut over = disks(600, 1);
    over.push(every[0].clone());
    let refused = push_all(&agents, &token, &over, now).await;
    assert_eq!(refused, [StatusCode::BAD_REQUEST]);
    assert_eq!(samples_of(&pool, PVE_01).await, 1025);
    assert_eq!(
        warnings(&pool).await,
        [(
            Some("pve-01".to_owned()),
            format!("Refused an agent push from {PERMITTED}: MALFORMED.")
        )]
    );

    // An hour on, the series made then were not created in the last hour: there is room again.
    agents
        .clock
        .advance(TimeDelta::hours(1) + TimeDelta::seconds(10));
    let next = now + TimeDelta::hours(1) + TimeDelta::seconds(10);
    assert_eq!(
        push_all(&agents, &token, &disks(600, 1), next).await,
        [StatusCode::NO_CONTENT]
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_098_e_back_dated_points_cannot_create_series_past_the_cap(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    for round in 0..5 {
        // Every point as old as K8 allows, so it is out of any "last hour" 10 seconds on.
        let oldest = now + TimeDelta::seconds(10 * round) - TimeDelta::hours(1);
        let first = usize::try_from(round).unwrap() * 1024;
        let statuses = push_all(&agents, &token, &disks(first, 1024), oldest).await;
        let expected = if round == 0 {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::BAD_REQUEST
        };
        assert_eq!(statuses, [expected; 4], "round {round}");
        agents.clock.advance(TimeDelta::seconds(10));
    }
    assert_eq!(series_of(&pool, PVE_01).await.len(), 1024);
}
