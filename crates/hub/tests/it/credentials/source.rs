//! `credentials.source_secrets` through the public source-secret functions (SRC-008, PRJ-018,
//! SEC-003).

use std::fmt::Write as _;

use anima_hub::{
    fleet::credentials::{
        Error, Secret, clear_source_secret, load_source_secret, store_source_secret,
    },
    ids::{SourceAccountId, SourceSecretId},
};
use sqlx::PgPool;

use super::{secret, vault};

const ACCOUNT: &str = "0190a000-0000-7000-8020-000000000001";
const OTHER: &str = "0190a000-0000-7000-8020-000000000002";

fn account(text: &str) -> SourceAccountId {
    text.parse().unwrap()
}

/// A throwaway PEM-shaped value, assembled here so no key text sits in the source.
fn pem() -> String {
    let marker = ["-----BEGIN", "RSA", "PRIVATE", "KEY-----"].join(" ");
    format!("{marker}\nthrowaway-test-body\n-----END RSA PRIVATE KEY-----\n")
}

async fn store(pool: &PgPool, to: SourceAccountId, text: &str) -> SourceSecretId {
    let mut conn = pool.acquire().await.unwrap();
    store_source_secret(&mut conn, &vault(1), to, secret(text))
        .await
        .unwrap()
}

async fn load(pool: &PgPool, id: SourceSecretId, by: SourceAccountId) -> Result<Secret, Error> {
    load_source_secret(pool, &vault(1), (id, by)).await
}

#[sqlx::test(migrations = "./migrations")]
async fn src_008_secret_opens_for_its_account(pool: PgPool) {
    let id = store(&pool, account(ACCOUNT), &pem()).await;
    let opened = load(&pool, id, account(ACCOUNT)).await.unwrap();
    assert_eq!(opened.expose(), pem().as_bytes());
}

#[sqlx::test(migrations = "./migrations")]
async fn src_008_secret_does_not_open_for_another_account(pool: PgPool) {
    let id = store(&pool, account(ACCOUNT), &pem()).await;
    assert!(matches!(
        load(&pool, id, account(OTHER)).await,
        Err(Error::Decrypt)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn src_008_row_moved_to_another_account_fails_to_open(pool: PgPool) {
    let id = store(&pool, account(ACCOUNT), &pem()).await;
    sqlx::query("UPDATE credentials.source_secrets SET account_id = $2 WHERE id = $1")
        .bind(id)
        .bind(account(OTHER))
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        load(&pool, id, account(OTHER)).await,
        Err(Error::Decrypt)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn src_008_row_copied_to_another_account_fails_to_open(pool: PgPool) {
    let id = store(&pool, account(ACCOUNT), &pem()).await;
    let copy: SourceSecretId = sqlx::query_scalar(
        "INSERT INTO credentials.source_secrets (id, account_id, nonce, ciphertext, key_id) \
         SELECT $2, $3, nonce, ciphertext, key_id FROM credentials.source_secrets \
         WHERE id = $1 RETURNING id",
    )
    .bind(id)
    .bind(uuid::Uuid::now_v7())
    .bind(account(OTHER))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(matches!(
        load(&pool, copy, account(OTHER)).await,
        Err(Error::Decrypt)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_003_table_holds_no_plaintext(pool: PgPool) {
    store(&pool, account(ACCOUNT), &pem()).await;
    let (nonce, ciphertext, key_id, text): (Vec<u8>, Vec<u8>, String, String) = sqlx::query_as(
        "SELECT nonce, ciphertext, key_id, t::text FROM credentials.source_secrets t",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let raw = [nonce, ciphertext, key_id.into_bytes(), text.into_bytes()].concat();
    for needle in ["BEGIN RSA PRIVATE KEY", "throwaway-test-body"] {
        assert!(
            !raw.windows(needle.len()).any(|w| w == needle.as_bytes()),
            "the row holds plaintext"
        );
        let hex: String = needle.bytes().fold(String::new(), |mut hex, b| {
            write!(hex, "{b:02x}").unwrap();
            hex
        });
        assert!(
            !raw.windows(hex.len()).any(|w| w == hex.as_bytes()),
            "the row holds plaintext as hex"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn src_008_clear_removes_the_secret(pool: PgPool) {
    let id = store(&pool, account(ACCOUNT), &pem()).await;
    let mut conn = pool.acquire().await.unwrap();
    assert!(
        !clear_source_secret(&mut conn, (id, account(OTHER)))
            .await
            .unwrap(),
        "another account's reference deletes nothing"
    );
    assert!(load(&pool, id, account(ACCOUNT)).await.is_ok());
    assert!(
        clear_source_secret(&mut conn, (id, account(ACCOUNT)))
            .await
            .unwrap()
    );
    assert!(
        !clear_source_secret(&mut conn, (id, account(ACCOUNT)))
            .await
            .unwrap()
    );
    assert!(matches!(
        load(&pool, id, account(ACCOUNT)).await,
        Err(Error::UnknownSourceSecret)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn src_008_storing_again_replaces_the_value_and_keeps_the_id(pool: PgPool) {
    let first = store(&pool, account(ACCOUNT), "old token").await;
    let second = store(&pool, account(ACCOUNT), "new token").await;
    assert_eq!(first, second);
    let opened = load(&pool, second, account(ACCOUNT)).await.unwrap();
    assert_eq!(opened.expose(), b"new token");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM credentials.source_secrets")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}
