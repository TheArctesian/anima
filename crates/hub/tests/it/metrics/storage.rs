//! MET-026 to MET-031 through the hub's routes: a pushed batch's `drives` replaces the device's
//! drives and its absence (an older agent) leaves them; `Device.storage` shows them drive by
//! drive with their latest use and activity; the card's readings name the fullest mount and
//! count the other drives.

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::agent::{
    batches::{batch, point},
    harness::{Agents, PVE_01, pushing_now},
};

const STORAGE: &str = "query($id: ID!) { device(id: $id) {
    readings { diskPercent diskMount otherDrives }
    storage { drives { name kind sizeBytes mounted readBps writeBps busyPercent
        mounts { path usedBytes sizeBytes percent } } } } }";

/// The pop-os drives as an agent sends them: `/` on nvme1n1, three drives not mounted.
fn pop_os_drives() -> Value {
    json!([
        { "name": "nvme0n1", "kind": "nvme", "sizeBytes": 1_000_204_886_016_u64, "mounts": [] },
        { "name": "nvme1n1", "kind": "nvme", "sizeBytes": 1_000_204_886_016_u64, "mounts": ["/", "/boot/efi"] },
        { "name": "sda", "kind": "hdd", "sizeBytes": 24_000_277_250_048_u64, "mounts": [] },
        { "name": "sdb", "kind": "hdd", "sizeBytes": 8_001_563_222_016_u64, "mounts": [] }
    ])
}

/// A batch at `at` with the root and EFI use and nvme1n1's activity, and `drives` when given.
fn storage_batch(at: DateTime<Utc>, drives: Option<Value>) -> Value {
    let series = json!([
        { "kind": "disk_percent", "label": "/" },
        { "kind": "disk_bytes", "label": "/" },
        { "kind": "disk_percent", "label": "/boot/efi" },
        { "kind": "disk_read_bps", "label": "nvme1n1" },
        { "kind": "disk_write_bps", "label": "nvme1n1" },
        { "kind": "disk_busy_percent", "label": "nvme1n1" }
    ]);
    let means = json!([
        81.0,
        810_000_000_000.0_f64,
        6.0,
        1_048_576.0,
        524_288.0,
        12.5
    ]);
    let mut pushed = batch(PVE_01, series, vec![point(at, means)]);
    if let Some(drives) = drives {
        pushed["drives"] = drives;
    }
    pushed
}

async fn drive_names(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT name FROM metrics.drives WHERE device_id = $1::uuid ORDER BY name")
        .bind(PVE_01)
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_027_pushed_drives_replace_the_stored_ones_and_a_batch_without_them_keeps_them(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let at = |step: i64| now - TimeDelta::seconds(40 - 10 * step);

    let older_agent = storage_batch(at(0), None);
    assert_eq!(
        agents.push(&token, &older_agent).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(drive_names(&pool).await, Vec::<String>::new());

    let first = storage_batch(at(1), Some(pop_os_drives()));
    assert_eq!(agents.push(&token, &first).await.0, StatusCode::NO_CONTENT);
    assert_eq!(
        drive_names(&pool).await,
        ["nvme0n1", "nvme1n1", "sda", "sdb"]
    );

    let unchanged = storage_batch(at(2), None);
    assert_eq!(
        agents.push(&token, &unchanged).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(drive_names(&pool).await.len(), 4, "absent means unchanged");

    let fewer = json!([{ "name": "nvme1n1", "kind": "nvme", "sizeBytes": 1, "mounts": ["/"] }]);
    let replaced = storage_batch(at(3), Some(fewer));
    assert_eq!(
        agents.push(&token, &replaced).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(drive_names(&pool).await, ["nvme1n1"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_027_a_drive_list_that_breaks_k8_is_malformed_and_stores_nothing(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let hostile = json!([{ "name": "sda", "kind": "hdd", "sizeBytes": 1, "mounts": ["relative"] }]);
    let (status, _) = agents
        .push(
            &token,
            &storage_batch(now - TimeDelta::seconds(10), Some(hostile)),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(drive_names(&pool).await, Vec::<String>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_029_storage_lists_each_drive_with_its_mounts_and_activity_and_the_card_its_mount(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let pushed = storage_batch(now - TimeDelta::seconds(10), Some(pop_os_drives()));
    assert_eq!(agents.push(&token, &pushed).await.0, StatusCode::NO_CONTENT);

    let answer = agents.operate(STORAGE, json!({ "id": PVE_01 })).await;
    let device = &answer["data"]["device"];
    assert_eq!(
        device["readings"],
        json!({ "diskPercent": 81.0, "diskMount": "/", "otherDrives": 3 }),
        "{answer}"
    );
    let drives = device["storage"]["drives"].as_array().unwrap();
    let rows: Vec<(&Value, &Value, &Value)> = drives
        .iter()
        .map(|row| (&row["name"], &row["kind"], &row["mounted"]))
        .collect();
    assert_eq!(
        rows,
        [
            (&json!("nvme1n1"), &json!("NVME"), &json!(true)),
            (&json!("nvme0n1"), &json!("NVME"), &json!(false)),
            (&json!("sda"), &json!("HDD"), &json!(false)),
            (&json!("sdb"), &json!("HDD"), &json!(false)),
        ]
    );
    let system = &drives[0];
    assert_eq!(system["readBps"], json!(1_048_576.0));
    assert_eq!(system["busyPercent"], json!(12.5));
    assert_eq!(
        system["mounts"][0],
        json!({ "path": "/", "usedBytes": 810_000_000_000.0_f64, "sizeBytes": 1_000_000_000_000.0_f64, "percent": 81.0 })
    );
    assert_eq!(
        system["mounts"][1],
        json!({ "path": "/boot/efi", "usedBytes": null, "sizeBytes": null, "percent": 6.0 })
    );
    assert_eq!(drives[2]["sizeBytes"], json!(24_000_277_250_048.0_f64));
    assert_eq!(drives[2]["mounts"], json!([]));
    assert_eq!(drives[2]["readBps"], Value::Null, "no activity yet");
}
