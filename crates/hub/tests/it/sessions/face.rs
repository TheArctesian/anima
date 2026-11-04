//! The sessions face other domains and `main.rs` call: host keys and the stale-row close.

use std::collections::BTreeMap;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    DOCKER_HOST, ECDSA_KEY, ED25519, ED25519_FINGERPRINT, ED25519_KEY, OLLAMA, PVE_01, Row,
    add_operator, insert_all, now, put_host_key, session_id,
};
use anima_hub::{
    ids::DeviceId,
    sessions::{self, Error},
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
        sessions::host_key(&pool, device(PVE_01))
            .await
            .unwrap()
            .is_none()
    );

    put_host_key(&pool, PVE_01, ED25519_KEY).await.unwrap();
    let key = sessions::host_key(&pool, device(PVE_01))
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

    let keys = sessions::host_keys(
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
    assert!(sessions::host_keys(&pool, &[]).await.unwrap().is_empty());
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
    let error = sessions::host_key(&pool, device(PVE_01)).await.unwrap_err();
    assert!(matches!(error, Error::Database(_)));
    assert_eq!(error.to_string(), "database error");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_close_stale_ends_only_open_sessions_with_hub_shutdown(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::live(1, PVE_01, 0),
            Row::live(2, DOCKER_HOST, 1),
            Row::ended(3, PVE_01, 2, "operator"),
        ],
    )
    .await;
    let at = now() + TimeDelta::hours(1);

    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 2);

    let rows: Vec<(uuid::Uuid, chrono::DateTime<chrono::Utc>, String)> =
        sqlx::query_as("select id, ended_at, end_reason from sessions.sessions order by id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [
            (session_id(1), at, "hub_shutdown".to_owned()),
            (session_id(2), at, "hub_shutdown".to_owned()),
            (
                session_id(3),
                now() + TimeDelta::minutes(7),
                "operator".to_owned()
            ),
        ]
    );
    assert_eq!(sessions::close_stale(&pool, at).await.unwrap(), 0);
}
