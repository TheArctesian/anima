//! Rows changed in the database do not open as another secret (SEC-003).

use anima_hub::credentials::{Error, load_ssh};
use sqlx::PgPool;

use super::{DOCKER_HOST, PASSPHRASE, PVE_01, device, password, private_key, store, vault};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_row_moved_to_another_device_fails_to_open(pool: PgPool) {
    let (vault, from, to) = (vault(1), device(PVE_01), device(DOCKER_HOST));
    store(&pool, &vault, from, password()).await.unwrap();
    sqlx::query("UPDATE credentials.secrets SET device_id = $2 WHERE device_id = $1")
        .bind(from)
        .bind(to)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        load_ssh(&pool, &vault, to).await,
        Err(Error::Decrypt)
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn passphrase_without_its_key_is_malformed(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM credentials.secrets WHERE device_id = $1 AND kind = 'ssh_private_key'",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        load_ssh(&pool, &vault, id).await,
        Err(Error::Malformed)
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_other_vault_key_fails(pool: PgPool) {
    let id = device(PVE_01);
    store(&pool, &vault(1), id, password()).await.unwrap();
    assert!(matches!(
        load_ssh(&pool, &vault(2), id).await,
        Err(Error::WrongKey)
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_003_kind_relabelled_fails_to_open(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM credentials.secrets WHERE device_id = $1 AND kind = 'ssh_private_key'",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE credentials.secrets SET kind = 'ssh_private_key' WHERE device_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        load_ssh(&pool, &vault, id).await,
        Err(Error::Decrypt)
    ));
}
