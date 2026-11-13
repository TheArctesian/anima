//! What a store writes and what a load gives back (SEC-003).

use anima_hub::fleet::credentials::{
    PasswordKind, SshSecret, clear_ssh, has_ssh, load_password, load_ssh,
};
use sqlx::PgPool;

use super::{
    DOCKER_HOST, HOST, KEY, PASSPHRASE, PASSWORD, PVE_01, RDP_PASSWORD, VNC_PASSWORD, device,
    kinds, password, private_key, secret, store, store_pw, vault,
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_stored_row_holds_no_plaintext(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();

    let (nonce, ciphertext, key_id): (Vec<u8>, Vec<u8>, String) = sqlx::query_as(
        "SELECT nonce, ciphertext, key_id FROM credentials.secrets WHERE device_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let plaintext = PASSWORD.as_bytes();
    assert_eq!(nonce.len(), 24);
    assert_ne!(ciphertext, plaintext);
    assert!(!ciphertext.windows(plaintext.len()).any(|w| w == plaintext));
    assert_eq!(key_id, vault.seal(&secret(""), b"").unwrap().key_id());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_password_round_trips(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();
    let loaded = load_ssh(&pool, &vault, (id, HOST)).await.unwrap();
    assert!(matches!(loaded, Some(SshSecret::Password(p)) if p.expose() == PASSWORD.as_bytes()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_private_key_round_trips(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(None)).await.unwrap();
    let loaded = load_ssh(&pool, &vault, (id, HOST)).await.unwrap();
    assert!(matches!(
        loaded,
        Some(SshSecret::PrivateKey { key, passphrase: None }) if key.expose() == KEY.as_bytes()
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_private_key_with_passphrase_round_trips(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
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
async fn sec_003_password_replaces_key_and_passphrase(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    assert_eq!(
        kinds(&pool, id).await,
        ["ssh_passphrase", "ssh_private_key"]
    );

    store(&pool, &vault, id, password()).await.unwrap();
    assert_eq!(kinds(&pool, id).await, ["ssh_password"]);
    let loaded = load_ssh(&pool, &vault, (id, HOST)).await.unwrap();
    assert!(matches!(loaded, Some(SshSecret::Password(p)) if p.expose() == PASSWORD.as_bytes()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn clear_ssh_removes_the_secret_once(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    assert!(!has_ssh(&pool, id).await.unwrap());
    assert!(load_ssh(&pool, &vault, (id, HOST)).await.unwrap().is_none());

    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    assert!(has_ssh(&pool, id).await.unwrap());
    assert!(!has_ssh(&pool, device(DOCKER_HOST)).await.unwrap());

    let mut conn = pool.acquire().await.unwrap();
    assert!(clear_ssh(&mut conn, id).await.unwrap());
    assert!(!clear_ssh(&mut conn, id).await.unwrap());
    assert!(!has_ssh(&pool, id).await.unwrap());
    assert!(load_ssh(&pool, &vault, (id, HOST)).await.unwrap().is_none());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_each_password_kind_round_trips_and_is_replaced(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    for (kind, text) in [
        (PasswordKind::Rdp, RDP_PASSWORD),
        (PasswordKind::Vnc, VNC_PASSWORD),
    ] {
        assert!(
            load_password(&pool, &vault, (id, HOST), kind)
                .await
                .unwrap()
                .is_none()
        );
        store_pw(&pool, id, kind, "first").await.unwrap();
        store_pw(&pool, id, kind, text).await.unwrap();
        let loaded = load_password(&pool, &vault, (id, HOST), kind)
            .await
            .unwrap();
        assert!(
            loaded.is_some_and(|p| p.expose() == text.as_bytes()),
            "{kind:?}"
        );
    }
    assert_eq!(kinds(&pool, id).await, ["rdp_password", "vnc_password"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_stored_password_row_holds_no_plaintext(pool: PgPool) {
    let id = device(PVE_01);
    store_pw(&pool, id, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    let ciphertext: Vec<u8> =
        sqlx::query_scalar("SELECT ciphertext FROM credentials.secrets WHERE device_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let plaintext = RDP_PASSWORD.as_bytes();
    assert!(!ciphertext.windows(plaintext.len()).any(|w| w == plaintext));
}
