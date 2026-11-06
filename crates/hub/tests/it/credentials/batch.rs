//! Which devices of a list have an SSH secret or a password of a kind, in one read (API-005).

use std::collections::HashSet;

use anima_hub::{
    credentials::{PasswordKind, with_password, with_ssh},
    ids::DeviceId,
};
use sqlx::PgPool;

use super::{
    DOCKER_HOST, PASSPHRASE, PVE_01, RDP_PASSWORD, VNC_PASSWORD, device, password, private_key,
    store, store_pw, vault,
};

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

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn with_password_of_no_devices_is_empty(pool: PgPool) {
    store_pw(&pool, device(PVE_01), PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    assert_eq!(
        with_password(&pool, &[], PasswordKind::Rdp).await.unwrap(),
        HashSet::new()
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn with_password_names_only_devices_with_that_kind(pool: PgPool) {
    let (rdp, vnc, both) = (device(PVE_01), device(DOCKER_HOST), device(OLLAMA));
    store_pw(&pool, rdp, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    store_pw(&pool, vnc, PasswordKind::Vnc, VNC_PASSWORD)
        .await
        .unwrap();
    for kind in [PasswordKind::Rdp, PasswordKind::Vnc] {
        store_pw(&pool, both, kind, "shared").await.unwrap();
    }
    let unknown = DeviceId::from(uuid::Uuid::now_v7());

    let asked = [rdp, vnc, both, unknown, rdp];
    assert_eq!(
        with_password(&pool, &asked, PasswordKind::Rdp)
            .await
            .unwrap(),
        HashSet::from([rdp, both])
    );
    assert_eq!(
        with_password(&pool, &asked, PasswordKind::Vnc)
            .await
            .unwrap(),
        HashSet::from([vnc, both])
    );
}
