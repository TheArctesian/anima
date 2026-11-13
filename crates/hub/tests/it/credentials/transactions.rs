//! `store_ssh` and `store_password` in the caller's connection: all or nothing, and the
//! caller's transaction survives.

use std::error::Error as _;

use anima_hub::{
    fleet::credentials::{
        Error, PasswordKind, SshSecret, has_ssh, load_password, load_ssh, store_password, store_ssh,
    },
    ids::DeviceId,
};
use sqlx::{Connection as _, PgPool};

use super::{
    HOST, PASSWORD, PVE_01, RDP_PASSWORD, device, kinds, password, private_key, secret, store,
    store_pw, vault,
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn store_for_unknown_device_fails(pool: PgPool) {
    let unknown = DeviceId::from(uuid::Uuid::now_v7());
    let result = store(&pool, &vault(1), unknown, password()).await;
    assert!(matches!(result, Err(Error::UnknownDevice)));
    assert_eq!(kinds(&pool, unknown).await, Vec::<String>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn store_failure_keeps_the_callers_transaction_usable(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut tx = pool.begin().await.unwrap();
    let unknown = DeviceId::from(uuid::Uuid::now_v7());
    let failed = store_ssh(&mut tx, &vault, (unknown, HOST), password()).await;
    assert!(matches!(failed, Err(Error::UnknownDevice)));
    store_ssh(&mut tx, &vault, (id, HOST), password())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(has_ssh(&pool, id).await.unwrap());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn store_in_a_rolled_back_transaction_leaves_nothing(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut conn = pool.acquire().await.unwrap();
    let mut tx = conn.begin().await.unwrap();
    store_ssh(&mut tx, &vault, (id, HOST), password())
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert!(!has_ssh(&pool, id).await.unwrap());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_failed_store_keeps_the_previous_secret_and_reveals_no_plaintext(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();
    // A trigger in this domain's schema makes the insert after the delete fail; its message
    // quotes the whole new row, so the error chain shows what the database saw.
    sqlx::query(
        "CREATE FUNCTION credentials.refuse_insert() RETURNS trigger LANGUAGE plpgsql \
         AS $$ BEGIN RAISE EXCEPTION 'refused %', NEW; END $$",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER refuse_insert BEFORE INSERT ON credentials.secrets \
         FOR EACH ROW EXECUTE FUNCTION credentials.refuse_insert()",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = store(&pool, &vault, id, private_key(Some("new passphrase")))
        .await
        .unwrap_err();
    let chain = format!("{error} | {error:?} | {}", error.source().unwrap());
    assert!(matches!(error, Error::Database(_)));
    assert!(chain.contains("refused"));
    assert!(!chain.contains("new passphrase") && !chain.contains("OPENSSH"));

    sqlx::query("DROP TRIGGER refuse_insert ON credentials.secrets")
        .execute(&pool)
        .await
        .unwrap();
    let loaded = load_ssh(&pool, &vault, (id, HOST)).await.unwrap();
    assert!(matches!(loaded, Some(SshSecret::Password(p)) if p.expose() == PASSWORD.as_bytes()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn password_store_for_unknown_device_keeps_the_callers_transaction(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let unknown = DeviceId::from(uuid::Uuid::now_v7());
    let mut tx = pool.begin().await.unwrap();
    let failed = store_password(
        &mut tx,
        &vault,
        (unknown, HOST),
        (PasswordKind::Rdp, secret("x")),
    )
    .await;
    assert!(matches!(failed, Err(Error::UnknownDevice)));
    store_password(
        &mut tx,
        &vault,
        (id, HOST),
        (PasswordKind::Rdp, secret(RDP_PASSWORD)),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(kinds(&pool, unknown).await, Vec::<String>::new());
    assert_eq!(kinds(&pool, id).await, ["rdp_password"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn log_007_failed_password_store_keeps_the_previous_password(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store_pw(&pool, id, PasswordKind::Rdp, RDP_PASSWORD)
        .await
        .unwrap();
    sqlx::query(
        "CREATE FUNCTION credentials.refuse_insert() RETURNS trigger LANGUAGE plpgsql \
         AS $$ BEGIN RAISE EXCEPTION 'refused %', NEW; END $$",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER refuse_insert BEFORE INSERT ON credentials.secrets \
         FOR EACH ROW EXECUTE FUNCTION credentials.refuse_insert()",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = store_pw(&pool, id, PasswordKind::Rdp, "new rdp password")
        .await
        .unwrap_err();
    let chain = format!("{error} | {error:?} | {}", error.source().unwrap());
    assert!(matches!(error, Error::Database(_)));
    assert!(chain.contains("refused") && !chain.contains("new rdp password"));

    sqlx::query("DROP TRIGGER refuse_insert ON credentials.secrets")
        .execute(&pool)
        .await
        .unwrap();
    let loaded = load_password(&pool, &vault, (id, HOST), PasswordKind::Rdp)
        .await
        .unwrap();
    assert!(loaded.is_some_and(|p| p.expose() == RDP_PASSWORD.as_bytes()));
}
