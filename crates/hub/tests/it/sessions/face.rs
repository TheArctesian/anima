//! The trust face sessions call: host keys (moved from sessions, Q2-05). The stale-row close
//! and the shutdown are tested with the other ends (`end/shutdown.rs`).

use std::collections::BTreeMap;

use sqlx::PgPool;

use super::{
    DOCKER_HOST, ECDSA_KEY, ED25519, ED25519_FINGERPRINT, ED25519_KEY, OLLAMA, PVE_01,
    add_operator, now, put_host_key,
};
use anima_hub::{
    ids::DeviceId,
    trust::{self, Error},
};

fn device(id: &str) -> DeviceId {
    id.parse().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_006_host_key_is_none_until_one_is_accepted(pool: PgPool) {
    add_operator(&pool).await;
    assert!(
        trust::host_key(&pool, device(PVE_01))
            .await
            .unwrap()
            .is_none()
    );

    put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap();
    let key = trust::host_key(&pool, device(PVE_01))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(key.public_key.algorithm(), "ssh-ed25519");
    assert_eq!(key.public_key.fingerprint().as_str(), ED25519_FINGERPRINT);
    assert_eq!(key.public_key.to_base64(), ED25519);
    assert_eq!(key.first_seen_at, now());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_005_host_keys_returns_each_device_that_has_one(pool: PgPool) {
    add_operator(&pool).await;
    put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap();
    put_host_key(&pool, DOCKER_HOST, ECDSA_KEY).await.unwrap();

    let keys = trust::host_keys(
        &pool,
        &[device(PVE_01), device(DOCKER_HOST), device(OLLAMA)],
    )
    .await
    .unwrap();
    let algorithms: BTreeMap<String, &str> = keys
        .iter()
        .map(|(id, key)| (id.to_string(), key.public_key.algorithm()))
        .collect();
    assert_eq!(
        algorithms,
        BTreeMap::from([
            (PVE_01.to_owned(), "ssh-ed25519"),
            (DOCKER_HOST.to_owned(), "ecdsa-sha2-nistp256"),
        ])
    );
    assert!(trust::host_keys(&pool, &[]).await.unwrap().is_empty());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn sec_007_a_stored_key_whose_algorithm_differs_from_the_blob_is_an_error(pool: PgPool) {
    add_operator(&pool).await;
    // The table checks the name's shape only; the hub checks it names the key's algorithm.
    put_host_key(&pool, PVE_01, ("ssh-rsa", ED25519, ED25519_FINGERPRINT))
        .await
        .unwrap();
    let error = trust::host_key(&pool, device(PVE_01)).await.unwrap_err();
    assert!(matches!(error, Error::Database(_)));
    assert_eq!(error.to_string(), "database error");
}
