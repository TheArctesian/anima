//! The table's named constraints and its link to the device (DEV-015).

use sqlx::PgPool;

use super::{
    PASSPHRASE, PVE_01, constraint_of, device, kinds, password, private_key, store, vault,
};

async fn insert_raw(pool: &PgPool, id: anima_hub::ids::DeviceId, kind: &str) -> sqlx::Error {
    sqlx::query(
        "INSERT INTO credentials.secrets (id, device_id, kind, nonce, ciphertext, key_id) \
         VALUES ($1, $2, $3, $4, $5, 'k')",
    )
    .bind(uuid::Uuid::now_v7())
    .bind(id)
    .bind(kind)
    .bind([0_u8; 24].as_slice())
    .bind([0_u8; 16].as_slice())
    .execute(pool)
    .await
    .unwrap_err()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn second_row_of_one_kind_fails_on_the_unique_constraint(pool: PgPool) {
    let id = device(PVE_01);
    store(&pool, &vault(1), id, password()).await.unwrap();
    let error = insert_raw(&pool, id, "ssh_password").await;
    assert_eq!(constraint_of(&error), Some("secrets_device_id_kind_key"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn unknown_kind_fails_on_the_kind_check(pool: PgPool) {
    let error = insert_raw(&pool, device(PVE_01), "totp").await;
    assert_eq!(constraint_of(&error), Some("secrets_kind_check"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn dev_015_deleting_the_device_deletes_its_secrets(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(Some(PASSPHRASE)))
        .await
        .unwrap();
    // devices exposes no delete function yet; this deletes the row the way it will.
    sqlx::query("DELETE FROM devices.devices WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(kinds(&pool, id).await, Vec::<String>::new());
}
