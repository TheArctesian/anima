//! Concurrent stores for one device are serialised, so no mixed row set survives.

use std::time::Duration;

use anima_hub::credentials::{SshSecret, load_ssh, store_ssh};
use sqlx::PgPool;

use super::{KEY, PVE_01, device, kinds, password, private_key, vault};

/// Upper bound for waiting until the second store queues on the device lock.
const BLOCK_WAIT: Duration = Duration::from_secs(10);

/// Whether a session waits for an advisory lock in this test's database (only the second
/// store can; `pg_locks` spans the whole server).
async fn advisory_lock_waiter(pool: &PgPool) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_locks l JOIN pg_database d ON d.oid = l.database \
         WHERE d.datname = current_database() AND l.locktype = 'advisory' AND NOT l.granted)",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn concurrent_stores_for_one_device_are_serialised(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut first = pool.begin().await.unwrap();
    store_ssh(&mut first, &vault, id, password()).await.unwrap();

    let second = tokio::spawn({
        let (pool, vault) = (pool.clone(), vault.clone());
        async move {
            let mut tx = pool.begin().await.unwrap();
            store_ssh(&mut tx, &vault, id, private_key(None))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
    });
    // Synchronise on the database's own lock table, not on a sleep: the bound only stops a
    // broken lock from hanging the test.
    tokio::time::timeout(BLOCK_WAIT, async {
        while !advisory_lock_waiter(&pool).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the second store must wait on the device lock");
    assert!(!second.is_finished());

    first.commit().await.unwrap();
    second.await.unwrap();

    assert_eq!(kinds(&pool, id).await, ["ssh_private_key"]);
    let loaded = load_ssh(&pool, &vault, id).await.unwrap();
    assert!(matches!(
        loaded,
        Some(SshSecret::PrivateKey { key, passphrase: None }) if key.expose() == KEY.as_bytes()
    ));
}
