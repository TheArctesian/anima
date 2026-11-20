//! API-005: `statuses`, `uptimes` and `resolved_ips` cost the same few statements for 13
//! devices as for 163.
//!
//! A TCP proxy between the pool and PostgreSQL counts the client's Bind and simple Query
//! messages, as in the devices batching test.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anima_hub::{
    fleet::checks::{self, UptimeWindow},
    ids::DeviceId,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// A pool on the same database whose statements are counted.
pub(in crate::checks) async fn counting_pool(hub: &PgPool) -> (PgPool, Arc<AtomicUsize>) {
    crate::transport::postgres::counting_pool(hub, 1).await
}

/// 150 more devices on the home site, each with SSH settings and two checks.
pub(in crate::checks) async fn add_bulk(pool: &PgPool) {
    for sql in [
        "INSERT INTO devices.devices (id, site_id, name, host, kind, os, created_at, updated_at) \
         SELECT gen_random_uuid(), '0190a000-0000-7000-8001-000000000001', 'bulk-' || n, \
                'bulk-' || n || '.lan', 'Server', 'linux', now(), now() \
         FROM generate_series(1, 150) AS n",
        "INSERT INTO devices.ssh_settings (device_id, enabled, port, username, auth) \
         SELECT id, false, 22, 'operator', 'password' FROM devices.devices WHERE name LIKE 'bulk-%'",
        "INSERT INTO checks.checks (id, device_id, type, name, status) \
         SELECT gen_random_uuid(), d.id, t.type, t.type, 'down' FROM devices.devices d, \
                unnest(ARRAY['icmp', 'tcp']) AS t(type) WHERE d.name LIKE 'bulk-%'",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

async fn all_devices(pool: &PgPool) -> Vec<DeviceId> {
    sqlx::query_scalar("SELECT id FROM devices.devices")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The statements that `statuses`, `uptimes` and `resolved_ips` cost for the devices.
async fn costs(pool: &PgPool, count: &AtomicUsize, devices: &[DeviceId]) -> [usize; 3] {
    let now: DateTime<Utc> = "2026-11-04T09:00:00Z".parse().unwrap();
    count.store(0, Ordering::SeqCst);
    let shown = checks::statuses(pool, devices, now).await.unwrap();
    assert_eq!(shown.len(), devices.len());
    let statuses = count.swap(0, Ordering::SeqCst);
    checks::uptimes(pool, devices, UptimeWindow::Week, now)
        .await
        .unwrap();
    let uptimes = count.swap(0, Ordering::SeqCst);
    checks::resolved_ips(pool, devices).await.unwrap();
    [statuses, uptimes, count.swap(0, Ordering::SeqCst)]
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(
        path = "../../../fixtures",
        scripts("devices", "checks", "maintenance")
    )
)]
async fn api_005_status_uptime_and_addresses_cost_constant_statements_for_13_or_163_devices(
    pool: PgPool,
) {
    let (counted, count) = counting_pool(&pool).await;
    let few = all_devices(&pool).await;
    costs(&counted, &count, &few).await;
    let few_cost = costs(&counted, &count, &few).await;

    add_bulk(&pool).await;
    let many = all_devices(&pool).await;
    let many_cost = costs(&counted, &count, &many).await;

    assert_eq!((few.len(), many.len()), (13, 163));
    assert_eq!(few_cost, many_cost);
    // One statement for the checks, one for the devices and one for the windows; one each
    // for the uptime sums and the addresses.
    assert_eq!(many_cost, [3, 1, 1]);
}
