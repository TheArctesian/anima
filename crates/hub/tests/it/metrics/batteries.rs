//! MET-032 through the hub's routes: a pushed batch's `batteries` replaces the device's
//! batteries and its absence leaves them; `Device.batteries` gives them by name and the charge
//! is the series `BATTERY_PERCENT` labelled with the battery name.

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::daemon::{
    batches::{batch, point},
    harness::{Daemons, PVE_01, pushing_now},
};

const BATTERIES: &str = "query($id: ID!) { device(id: $id) {
    batteries { name state secondsLeft healthPercent }
    metrics(range: HOUR) { kind label } } }";

/// A batch at `at` with the charge of BAT0, and `batteries` when given.
fn battery_batch(at: DateTime<Utc>, batteries: Option<Value>) -> Value {
    let series = json!([{ "kind": "battery_percent", "label": "BAT0" }]);
    let mut pushed = batch(PVE_01, series, vec![point(at, json!([85.0]))]);
    if let Some(batteries) = batteries {
        pushed["batteries"] = batteries;
    }
    pushed
}

async fn battery_states(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT name, state FROM metrics.batteries WHERE device_id = $1::uuid ORDER BY name",
    )
    .bind(PVE_01)
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_032_pushed_batteries_replace_the_stored_ones_and_a_batch_without_them_keeps_them(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let at = |step: i64| now - TimeDelta::seconds(40 - 10 * step);

    let first = json!([
        { "name": "BAT0", "state": "discharging", "secondsLeft": 9000, "healthPercent": 91 },
        { "name": "BAT1", "state": "overheating" }
    ]);
    let pushed = battery_batch(at(0), Some(first));
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
        StatusCode::NO_CONTENT
    );
    let stored = [
        ("BAT0".to_owned(), "discharging".to_owned()),
        ("BAT1".to_owned(), "unknown".to_owned()),
    ];
    assert_eq!(battery_states(&pool).await, stored);

    let unchanged = battery_batch(at(1), None);
    assert_eq!(
        daemons.push(&token, &unchanged).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        battery_states(&pool).await,
        stored,
        "absent means unchanged"
    );

    let gone = battery_batch(at(2), Some(json!([])));
    assert_eq!(daemons.push(&token, &gone).await.0, StatusCode::NO_CONTENT);
    assert_eq!(battery_states(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_032_a_battery_list_that_breaks_k8_is_malformed_and_stores_nothing(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let hostile = json!([{ "name": "BAT0", "state": "full", "healthPercent": 120 }]);
    let pushed = battery_batch(now - TimeDelta::seconds(10), Some(hostile));
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(battery_states(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_032_the_device_gives_its_batteries_and_the_charge_series(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let listed = json!([{ "name": "BAT0", "state": "not_charging", "healthPercent": 91 }]);
    let pushed = battery_batch(now - TimeDelta::seconds(10), Some(listed));
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
        StatusCode::NO_CONTENT
    );

    let answer = daemons.operate(BATTERIES, json!({ "id": PVE_01 })).await;
    let device = &answer["data"]["device"];
    assert_eq!(
        device["batteries"],
        json!([{ "name": "BAT0", "state": "NOT_CHARGING", "secondsLeft": null, "healthPercent": 91 }]),
        "{answer}"
    );
    assert_eq!(
        device["metrics"],
        json!([{ "kind": "BATTERY_PERCENT", "label": "BAT0" }])
    );
}
