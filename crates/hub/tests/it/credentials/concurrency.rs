//! Concurrent writers for one device, of any kind, are serialised, so no mixed row set
//! survives; writers for different devices do not wait for each other.

use std::time::Duration;

use anima_hub::credentials::{
    PasswordKind, SshSecret, clear_password, clear_ssh, load_password, load_ssh, store_password,
    store_ssh,
};
use sqlx::PgPool;

use super::{
    DOCKER_HOST, KEY, PVE_01, RDP_PASSWORD, VNC_PASSWORD, device, kinds, password, private_key,
    secret, store, vault,
};

/// Upper bound for any wait in these tests, so a broken lock fails instead of hanging.
const BLOCK_WAIT: Duration = Duration::from_secs(10);

/// Advisory locks held or awaited in this test's database (`pg_locks` spans the server).
async fn advisory_locks(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_locks l JOIN pg_database d ON d.oid = l.database \
         WHERE d.datname = current_database() AND l.locktype = 'advisory'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Whether a session in this test's database waits for any lock, advisory or row
/// (`pg_locks` shows no row-lock waits; `pg_stat_activity` does).
async fn lock_waiter(pool: &PgPool) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_stat_activity \
         WHERE datname = current_database() AND wait_event_type = 'Lock')",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

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
    tokio::time::timeout(BLOCK_WAIT, second)
        .await
        .expect("the second store must finish once the first commits")
        .unwrap();

    assert_eq!(kinds(&pool, id).await, ["ssh_private_key"]);
    let loaded = load_ssh(&pool, &vault, id).await.unwrap();
    assert!(matches!(
        loaded,
        Some(SshSecret::PrivateKey { key, passphrase: None }) if key.expose() == KEY.as_bytes()
    ));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn no_device_lock_outlives_its_transaction(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut tx = pool.begin().await.unwrap();
    store_ssh(&mut tx, &vault, id, password()).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    assert!(clear_ssh(&mut tx, id).await.unwrap());
    tx.commit().await.unwrap();

    store(&pool, &vault, id, password()).await.unwrap();
    assert!(
        clear_ssh(&mut pool.acquire().await.unwrap(), id)
            .await
            .unwrap()
    );

    assert_eq!(advisory_locks(&pool).await, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn clear_waits_for_a_concurrent_store(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, password()).await.unwrap();
    let mut first = pool.begin().await.unwrap();
    store_ssh(&mut first, &vault, id, private_key(None))
        .await
        .unwrap();

    let clear = tokio::spawn({
        let pool = pool.clone();
        async move {
            clear_ssh(&mut pool.acquire().await.unwrap(), id)
                .await
                .unwrap()
        }
    });
    tokio::time::timeout(BLOCK_WAIT, async {
        while !lock_waiter(&pool).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the clear must wait for the uncommitted store");
    assert!(!clear.is_finished());

    first.commit().await.unwrap();
    let removed = tokio::time::timeout(BLOCK_WAIT, clear)
        .await
        .expect("the clear must finish once the store commits")
        .unwrap();
    assert!(removed);
    assert_eq!(kinds(&pool, id).await, Vec::<String>::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn stores_for_different_devices_do_not_wait(pool: PgPool) {
    let vault = vault(1);
    let mut first = pool.begin().await.unwrap();
    store_ssh(&mut first, &vault, device(PVE_01), password())
        .await
        .unwrap();

    tokio::time::timeout(
        BLOCK_WAIT,
        store(&pool, &vault, device(DOCKER_HOST), private_key(None)),
    )
    .await
    .expect("a store for another device must not wait for this lock")
    .unwrap();
    first.commit().await.unwrap();
    assert_eq!(kinds(&pool, device(DOCKER_HOST)).await, ["ssh_private_key"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn password_store_waits_for_a_concurrent_ssh_store(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut first = pool.begin().await.unwrap();
    store_ssh(&mut first, &vault, id, password()).await.unwrap();

    let second = tokio::spawn({
        let (pool, vault) = (pool.clone(), vault.clone());
        async move {
            let mut tx = pool.begin().await.unwrap();
            store_password(&mut tx, &vault, id, PasswordKind::Rdp, secret(RDP_PASSWORD))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
    });
    tokio::time::timeout(BLOCK_WAIT, async {
        while !advisory_lock_waiter(&pool).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the password store must wait on the device lock the SSH store holds");
    assert!(!second.is_finished());

    first.commit().await.unwrap();
    tokio::time::timeout(BLOCK_WAIT, second)
        .await
        .expect("the password store must finish once the SSH store commits")
        .unwrap();
    assert_eq!(kinds(&pool, id).await, ["rdp_password", "ssh_password"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ssh_clear_waits_for_a_concurrent_password_store(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    store(&pool, &vault, id, private_key(None)).await.unwrap();
    let mut first = pool.begin().await.unwrap();
    store_password(
        &mut first,
        &vault,
        id,
        PasswordKind::Vnc,
        secret(VNC_PASSWORD),
    )
    .await
    .unwrap();

    let clear = tokio::spawn({
        let pool = pool.clone();
        async move {
            let mut tx = pool.begin().await.unwrap();
            let removed = clear_ssh(&mut tx, id).await.unwrap();
            let rdp = clear_password(&mut tx, id, PasswordKind::Rdp)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            (removed, rdp)
        }
    });
    tokio::time::timeout(BLOCK_WAIT, async {
        while !lock_waiter(&pool).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the SSH clear must wait for the uncommitted password store");
    assert!(!clear.is_finished());

    first.commit().await.unwrap();
    let (removed, rdp) = tokio::time::timeout(BLOCK_WAIT, clear)
        .await
        .expect("the clear must finish once the password store commits")
        .unwrap();
    assert!(removed && !rdp);
    assert_eq!(kinds(&pool, id).await, ["vnc_password"]);
    let vnc = load_password(&pool, &vault, id, PasswordKind::Vnc)
        .await
        .unwrap();
    assert!(vnc.is_some_and(|p| p.expose() == VNC_PASSWORD.as_bytes()));
    assert_eq!(advisory_locks(&pool).await, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn password_clear_waits_for_a_concurrent_password_store(pool: PgPool) {
    let (vault, id) = (vault(1), device(PVE_01));
    let mut first = pool.begin().await.unwrap();
    store_password(
        &mut first,
        &vault,
        id,
        PasswordKind::Rdp,
        secret(RDP_PASSWORD),
    )
    .await
    .unwrap();

    let clear = tokio::spawn({
        let pool = pool.clone();
        async move {
            clear_password(&mut pool.acquire().await.unwrap(), id, PasswordKind::Rdp)
                .await
                .unwrap()
        }
    });
    tokio::time::timeout(BLOCK_WAIT, async {
        while !advisory_lock_waiter(&pool).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the password clear must wait on the device lock");
    first.commit().await.unwrap();
    let removed = tokio::time::timeout(BLOCK_WAIT, clear)
        .await
        .expect("the clear must finish once the store commits")
        .unwrap();
    assert!(removed);
    assert_eq!(kinds(&pool, id).await, Vec::<String>::new());
}
