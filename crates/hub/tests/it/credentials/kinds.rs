//! SSH secrets and RDP and VNC passwords share the table but never each other's rows (D-8,
//! SEC-003, SEC-005).

use anima_hub::fleet::credentials::{
    PasswordKind, SshSecret, clear_password, clear_ssh, has_ssh, load_password, load_ssh,
    with_password, with_ssh,
};
use sqlx::PgPool;
use std::collections::HashSet;

use super::{
    DOCKER_HOST, HOST, KEY, PASSPHRASE, PASSWORD, PVE_01, RDP_PASSWORD, VNC_PASSWORD, device,
    kinds, password, private_key, store, store_pw, vault,
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_011_passwords_alone_are_no_ssh_secret(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store_pw(&pool, id, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    store_pw(&pool, id, PasswordKind::Vnc, VNC_PASSWORD)
        .await
        .unwrap();

    assert!(!has_ssh(&pool, id).await.unwrap());
    assert_eq!(with_ssh(&pool, &[id]).await.unwrap(), HashSet::new());
    assert!(load_ssh(&pool, &vault, (id, HOST)).await.unwrap().is_none());
    assert!(
        !clear_ssh(&mut pool.acquire().await.unwrap(), id)
            .await
            .unwrap()
    );
    assert_eq!(kinds(&pool, id).await, ["rdp_password", "vnc_password"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_011_ssh_store_and_clear_leave_passwords(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store_pw(&pool, id, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    store(&pool, &vault, id, password()).await.unwrap();

    let loaded = load_ssh(&pool, &vault, (id, HOST)).await.unwrap();
    assert!(matches!(loaded, Some(SshSecret::Password(p)) if p.expose() == PASSWORD.as_bytes()));
    assert_eq!(kinds(&pool, id).await, ["rdp_password", "ssh_password"]);

    assert!(
        clear_ssh(&mut pool.acquire().await.unwrap(), id)
            .await
            .unwrap()
    );
    assert_eq!(kinds(&pool, id).await, ["rdp_password"]);
    let rdp = load_password(&pool, &vault, (id, HOST), PasswordKind::Rdp)
        .await
        .unwrap();
    assert!(rdp.is_some_and(|p| p.expose() == RDP_PASSWORD.as_bytes()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_011_ssh_with_passwords_still_loads_as_ssh(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(None)).await.unwrap();
    store_pw(&pool, id, PasswordKind::Vnc, VNC_PASSWORD)
        .await
        .unwrap();
    let loaded = load_ssh(&pool, &vault, (id, HOST)).await.unwrap();
    assert!(matches!(
        loaded,
        Some(SshSecret::PrivateKey { key, passphrase: None }) if key.expose() == KEY.as_bytes()
    ));
    assert!(has_ssh(&pool, id).await.unwrap());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_011_ssh_batch_names_only_devices_with_ssh(pool: PgPool) {
    let vault = vault(1);
    let (ssh, rdp_only) = (device(PVE_01), device(DOCKER_HOST));
    store(&pool, &vault, ssh, password()).await.unwrap();
    store_pw(&pool, rdp_only, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    assert_eq!(
        with_ssh(&pool, &[ssh, rdp_only]).await.unwrap(),
        HashSet::from([ssh])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_011_password_functions_leave_ssh_and_the_other_kind(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    store_pw(&pool, id, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    store_pw(&pool, id, PasswordKind::Vnc, VNC_PASSWORD)
        .await
        .unwrap();
    assert_eq!(
        kinds(&pool, id).await,
        [
            "rdp_password",
            "ssh_passphrase",
            "ssh_private_key",
            "vnc_password"
        ]
    );

    let mut conn = pool.acquire().await.unwrap();
    assert!(
        clear_password(&mut conn, id, PasswordKind::Rdp)
            .await
            .unwrap()
    );
    assert!(
        !clear_password(&mut conn, id, PasswordKind::Rdp)
            .await
            .unwrap()
    );
    assert_eq!(
        kinds(&pool, id).await,
        ["ssh_passphrase", "ssh_private_key", "vnc_password"]
    );
    assert!(
        load_password(&pool, &vault, (id, HOST), PasswordKind::Rdp)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        with_password(&pool, &[id], PasswordKind::Rdp)
            .await
            .unwrap()
            .is_empty()
    );

    assert!(
        clear_password(&mut conn, id, PasswordKind::Vnc)
            .await
            .unwrap()
    );
    assert_eq!(
        kinds(&pool, id).await,
        ["ssh_passphrase", "ssh_private_key"]
    );
    let loaded = load_ssh(&pool, &vault, (id, HOST)).await.unwrap();
    assert!(matches!(
        loaded,
        Some(SshSecret::PrivateKey { key, passphrase: Some(p) })
            if key.expose() == KEY.as_bytes() && p.expose() == PASSPHRASE.as_bytes()
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_011_ssh_rows_are_no_password(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();
    for kind in [PasswordKind::Rdp, PasswordKind::Vnc] {
        assert!(
            load_password(&pool, &vault, (id, HOST), kind)
                .await
                .unwrap()
                .is_none()
        );
        assert!(with_password(&pool, &[id], kind).await.unwrap().is_empty());
        assert!(
            !clear_password(&mut pool.acquire().await.unwrap(), id, kind)
                .await
                .unwrap()
        );
    }
    assert_eq!(kinds(&pool, id).await, ["ssh_password"]);
}
