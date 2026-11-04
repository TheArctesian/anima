//! The public face of `devices` (K3) against the K6 rows.

use anima_hub::devices::{self, Error, Missing, OperatingSystem, SshAuth};
use anima_hub::ids::DeviceId;
use sqlx::PgPool;

fn id(text: &str) -> DeviceId {
    text.parse().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k3_get_returns_every_field_with_tags_and_ssh(pool: PgPool) {
    let device = devices::get(&pool, id(super::PVE_01)).await.unwrap();
    assert_eq!(device.name, "pve-01");
    assert_eq!(device.host, "pve-01.home.lan");
    assert_eq!(device.ip, Some("10.10.0.2".parse().unwrap()));
    assert_eq!(device.kind, "Proxmox VE node");
    assert_eq!(device.os, OperatingSystem::Linux);
    assert_eq!(device.tags, ["infra", "proxmox"]);
    assert!(!device.maintenance);
    assert_eq!(
        device.site.to_string(),
        "0190a000-0000-7000-8001-000000000001"
    );
    assert!(device.ssh.enabled);
    assert_eq!(device.ssh.port, 22);
    assert_eq!(device.ssh.username, "operator");
    assert_eq!(device.ssh.auth, SshAuth::PrivateKey);
    assert_eq!(device.created_at.to_rfc3339(), "2026-11-01T00:00:00+00:00");
    assert_eq!(device.updated_at, device.created_at);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k3_get_of_an_unknown_id_is_not_found(pool: PgPool) {
    let missing = id("0190a000-0000-7000-8002-0000000000f1");
    let error = devices::get(&pool, missing).await.unwrap_err();
    assert!(matches!(error, Error::NotFound(Missing::Device(found)) if found == missing));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k3_list_returns_all_devices_in_name_byte_order(pool: PgPool) {
    let all = devices::list(&pool).await.unwrap();
    let names: Vec<_> = all.iter().map(|device| device.name.as_str()).collect();
    assert_eq!(names.len(), 13);
    let mut sorted = names.clone();
    sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    assert_eq!(names, sorted);
    assert!(all.iter().all(|device| device.ssh.port == 22));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k3_in_maintenance_is_true_only_for_jellyfin(pool: PgPool) {
    assert!(
        devices::in_maintenance(&pool, id(super::JELLYFIN))
            .await
            .unwrap()
    );
    assert!(
        !devices::in_maintenance(&pool, id(super::PVE_01))
            .await
            .unwrap()
    );
    let missing = id("0190a000-0000-7000-8002-0000000000f1");
    assert!(matches!(
        devices::in_maintenance(&pool, missing).await,
        Err(Error::NotFound(_))
    ));
}
