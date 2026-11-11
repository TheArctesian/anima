//! K8 Section 4, R-098 b and f, R-099, MET-002, MET-006: a batch's `gpus` replaces the stored
//! set, `[]` empties it and its absence leaves it; a removed GPU's series keep their history. A
//! GPU series is keyed by its index alone; a GPU temperature or power past its ceiling makes the
//! batch `MALFORMED` (CR-17); and no finite value overflows an aggregate.

use axum::http::StatusCode;
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    batches::{aggregates, batch, gpus_of, point, samples_of, series_of},
    harness::{Agents, PVE_01, pushing_now, refusal},
};

fn gpu(index: u8, vendor: &str, has_vram: bool) -> Value {
    json!({ "index": index, "name": format!("GPU {index}"), "vendor": vendor, "hasVram": has_vram })
}

/// A batch of one `gpu_percent` point per GPU in `indexes` at `at`, with `gpus` when given.
fn gpu_batch(at: chrono::DateTime<chrono::Utc>, indexes: &[u8], gpus: Option<Value>) -> Value {
    let series: Vec<Value> = indexes
        .iter()
        .map(|index| json!({ "kind": "gpu_percent", "gpu": index }))
        .collect();
    let means: Vec<f64> = indexes
        .iter()
        .map(|index| f64::from(*index) + 1.0)
        .collect();
    let mut pushed = batch(PVE_01, json!(series), vec![point(at, json!(means))]);
    if let Some(gpus) = gpus {
        pushed["gpus"] = gpus;
    }
    pushed
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_098_b_gpus_present_replace_the_set_absent_keep_it_and_empty_clears_it(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let at = |step: i64| now - TimeDelta::seconds(40 - 10 * step);
    let both = json!([gpu(0, "nvidia", true), gpu(1, "amd", true)]);
    let pushes = [
        (gpu_batch(at(0), &[0, 1], Some(both)), vec![0, 1]),
        (gpu_batch(at(1), &[0, 1], None), vec![0, 1]),
        (
            gpu_batch(at(2), &[0], Some(json!([gpu(0, "nvidia", true)]))),
            vec![0],
        ),
        (gpu_batch(at(3), &[0], Some(json!([]))), vec![]),
    ];
    for (pushed, expected) in pushes {
        assert_eq!(agents.push(&token, &pushed).await.0, StatusCode::NO_CONTENT);
        assert_eq!(gpus_of(&pool, PVE_01).await, expected, "{pushed}");
    }
    // GPU 1 is gone from the set; its series and its two samples stay.
    assert_eq!(
        series_of(&pool, PVE_01).await,
        [
            ("gpu_percent".to_owned(), "0".to_owned(), Some(0)),
            ("gpu_percent".to_owned(), "1".to_owned(), Some(1)),
        ]
    );
    assert_eq!(samples_of(&pool, PVE_01).await, 6);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_098_b_the_wire_vendor_and_vram_reach_the_stored_gpu(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let gpus = json!([
        gpu(0, "nvidia", true),
        gpu(1, "amd", true),
        gpu(2, "intel", false),
        gpu(3, "apple", false),
    ]);
    let pushed = gpu_batch(now, &[0], Some(gpus));
    assert_eq!(agents.push(&token, &pushed).await.0, StatusCode::NO_CONTENT);
    let stored: Vec<(i32, String, String, bool)> = sqlx::query_as(
        "SELECT index, name, vendor, has_vram FROM metrics.gpus WHERE device_id = $1::uuid \
         ORDER BY index",
    )
    .bind(PVE_01)
    .fetch_all(&pool)
    .await
    .unwrap();
    let expected: Vec<(i32, String, String, bool)> = [
        ("nvidia", true),
        ("amd", true),
        ("intel", false),
        ("apple", false),
    ]
    .iter()
    .zip(0..)
    .map(|((vendor, vram), index)| (index, format!("GPU {index}"), (*vendor).to_owned(), *vram))
    .collect();
    assert_eq!(stored, expected);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn r_099_a_gpu_series_with_an_agent_chosen_label_is_refused_malformed(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    for series in [
        json!({ "kind": "gpu_percent", "gpu": 0, "label": "0" }),
        json!({ "kind": "gpu_percent", "gpu": 0, "label": "card0" }),
        json!({ "kind": "gpu_percent" }),
        json!({ "kind": "cpu_percent", "gpu": 0 }),
    ] {
        let pushed = batch(PVE_01, json!([series]), vec![point(now, json!([5.0]))]);
        let answer = agents.push(&token, &pushed).await;
        assert_eq!(
            answer,
            (StatusCode::BAD_REQUEST, refusal("MALFORMED")),
            "{series}"
        );
    }
    assert_eq!(series_of(&pool, PVE_01).await.len(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cr_17_gpu_temperature_and_power_at_their_ceiling_are_stored_and_past_it_malformed(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let series = json!([
        { "kind": "gpu_temperature_c", "gpu": 0 },
        { "kind": "gpu_power_w", "gpu": 0 },
    ]);
    let at_ceiling = vec![point(
        now - TimeDelta::seconds(10),
        json!([1000.0, 10_000.0]),
    )];
    let answer = agents
        .push(&token, &batch(PVE_01, series.clone(), at_ceiling))
        .await;
    assert_eq!(answer.0, StatusCode::NO_CONTENT, "{}", answer.1);
    for past in [json!([1_000.000_001, 5.0]), json!([5.0, 10_000.000_001])] {
        let pushed = batch(PVE_01, series.clone(), vec![point(now, past.clone())]);
        let answer = agents.push(&token, &pushed).await;
        assert_eq!(
            answer,
            (StatusCode::BAD_REQUEST, refusal("MALFORMED")),
            "{past}"
        );
    }
    let stored: Vec<(String, f64)> = sqlx::query_as(
        "SELECT s.kind, m.value FROM metrics.samples m JOIN metrics.series s \
         ON s.id = m.series_id ORDER BY s.kind",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored,
        [
            ("gpu_power_w".to_owned(), 10_000.0),
            ("gpu_temperature_c".to_owned(), 1000.0),
        ]
    );
    for kind in ["gpu_temperature_c", "gpu_power_w"] {
        let rows = aggregates(&pool, "minute", PVE_01, kind).await;
        let count: i32 = rows.iter().map(|row| row.3).sum();
        assert_eq!(count, 1, "{kind}: a refused value never reached the mean");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_002_the_largest_finite_values_fold_into_the_aggregates_without_overflow(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let series = json!([{ "kind": "memory_bytes" }]);
    let at = |back: i64| now - TimeDelta::seconds(back);
    let first = batch(
        PVE_01,
        series.clone(),
        vec![
            point(at(20), json!([f64::MAX])),
            point(at(10), json!([f64::MAX])),
        ],
    );
    let second = batch(PVE_01, series, vec![point(at(0), json!([f64::MAX]))]);
    assert_eq!(agents.push(&token, &first).await.0, StatusCode::NO_CONTENT);
    let answer = agents.push(&token, &second).await;
    assert_eq!(answer.0, StatusCode::NO_CONTENT, "{}", answer.1);
    for table in ["minute", "hour", "day"] {
        let count: i32 = aggregates(&pool, table, PVE_01, "memory_bytes")
            .await
            .iter()
            .map(|row| {
                assert!(row.0.is_finite() && row.0 <= f64::MAX, "{table}: {row:?}");
                assert_eq!(row.2, f64::MAX, "{table}");
                row.3
            })
            .sum();
        assert_eq!(count, 3, "{table}");
    }
}
