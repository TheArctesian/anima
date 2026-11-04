//! Which devices of a list have an SSH secret, in one read (API-005).

use std::collections::HashSet;

use anima_hub::{credentials::with_ssh, ids::DeviceId};
use sqlx::PgPool;

use super::{DOCKER_HOST, PASSPHRASE, PVE_01, device, password, private_key, store, vault};

const OLLAMA: &str = "0190a000-0000-7000-8002-000000000003";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn with_ssh_of_no_devices_is_empty(pool: PgPool) {
    store(&pool, &vault(1), device(PVE_01), password())
        .await
        .unwrap();
    assert_eq!(with_ssh(&pool, &[]).await.unwrap(), HashSet::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn with_ssh_names_each_device_with_a_secret_once(pool: PgPool) {
    let vault = vault(1);
    let (with_password, with_key) = (device(PVE_01), device(DOCKER_HOST));
    store(&pool, &vault, with_password, password())
        .await
        .unwrap();
    store(&pool, &vault, with_key, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();

    let asked = [with_password, with_key, device(OLLAMA), with_key];
    let found = with_ssh(&pool, &asked).await.unwrap();
    assert_eq!(found, HashSet::from([with_password, with_key]));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn with_ssh_leaves_out_an_unknown_device(pool: PgPool) {
    let known = device(PVE_01);
    store(&pool, &vault(1), known, password()).await.unwrap();
    let unknown = DeviceId::from(uuid::Uuid::now_v7());
    let found = with_ssh(&pool, &[unknown, known]).await.unwrap();
    assert_eq!(found, HashSet::from([known]));
}
