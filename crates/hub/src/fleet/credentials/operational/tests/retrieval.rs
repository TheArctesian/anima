//! Retrieval by a consumer's daemon: who gets a key, what the hub records of each daemon, and
//! what a removed device leaves (R-147, K-E5 Section 2.1).

use daemon_core::wire::KeyVersion;
use sqlx::PgPool;

use super::{Desk, Grant, database_now, docker_host, metadata, pve};
use crate::fleet::credentials::operational::{
    Error, current_for, grant, key, record_held, revoke, ungrant,
};

fn held(name: &str, version: u32) -> KeyVersion {
    KeyVersion {
        name: name.to_owned(),
        version,
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_no_grant_no_key_and_no_active_version_are_one_answer(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("granted", metadata(), now).await;
    desk.create("ungranted", metadata(), now).await;
    desk.create("revoked", metadata(), now).await;
    desk.grant("granted", pve(), now).await;
    desk.grant("revoked", pve(), now).await;
    let mut tx = pool.begin().await.unwrap();
    revoke(&mut tx, "revoked", desk.trail(now)).await.unwrap();
    tx.commit().await.unwrap();

    assert!(desk.fetch(pve(), "granted", now).await.is_some());
    for name in ["ungranted", "missing", "revoked", "Not A Name"] {
        assert!(desk.fetch(pve(), name, now).await.is_none(), "{name}");
    }
    assert!(desk.fetch(docker_host(), "granted", now).await.is_none());
    let fetches: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events.audit_log WHERE action = 'fetch' \
         AND target_type = 'operational_key'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(fetches, 1, "only the fetch that gave a value is audited");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_held_and_fetched_versions_are_recorded_and_shown(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("cloudflare.dns", metadata(), now).await;
    desk.create("other", metadata(), now).await;
    desk.grant("cloudflare.dns", pve(), now).await;
    desk.grant("other", pve(), now).await;
    desk.fetch(pve(), "cloudflare.dns", now).await.unwrap();

    record_held(&pool, pve(), &[held("cloudflare.dns", 1)], now)
        .await
        .unwrap();

    let shown = key(&pool, "cloudflare.dns", now).await.unwrap().unwrap();
    let consumer = &shown.consumers[0];
    assert_eq!(consumer.held_version, Some(1));
    assert_eq!(consumer.held_seen_at, Some(now));
    assert_eq!(
        (consumer.fetched_version, consumer.fetched_at),
        (Some(1), Some(now))
    );
    let other = key(&pool, "other", now).await.unwrap().unwrap();
    assert_eq!(
        other.consumers[0].held_version, None,
        "not named: holds none"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_current_lists_each_granted_key_once_with_its_active_version(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("b.key", metadata(), now).await;
    desk.create("a.key", metadata(), now).await;
    desk.create("not.mine", metadata(), now).await;
    desk.grant("b.key", pve(), now).await;
    desk.grant("a.key", pve(), now).await;
    desk.grant("not.mine", docker_host(), now).await;
    let second = Grant {
        name: "a.key".to_owned(),
        device: pve(),
        workload: "backup".to_owned(),
    };
    let mut tx = pool.begin().await.unwrap();
    grant(&mut tx, second, desk.trail(now)).await.unwrap();
    tx.commit().await.unwrap();
    desk.rotate("b.key", "two", now).await;

    let current = current_for(&pool, pve(), now).await.unwrap();

    assert_eq!(current, [held("a.key", 1), held("b.key", 2)]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_a_deleted_device_leaves_no_consumer_row(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("cloudflare.dns", metadata(), now).await;
    desk.grant("cloudflare.dns", pve(), now).await;
    desk.grant("cloudflare.dns", docker_host(), now).await;

    sqlx::query("DELETE FROM devices.devices WHERE id = $1")
        .bind(pve())
        .execute(&pool)
        .await
        .unwrap();

    let shown = key(&pool, "cloudflare.dns", now).await.unwrap().unwrap();
    let devices: Vec<_> = shown.consumers.iter().map(|c| c.device).collect();
    assert_eq!(devices, [docker_host()]);
    assert!(desk.fetch(pve(), "cloudflare.dns", now).await.is_none());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_a_grant_needs_a_device_and_an_ungrant_needs_a_grant(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("cloudflare.dns", metadata(), now).await;
    let nowhere = Grant {
        name: "cloudflare.dns".to_owned(),
        device: uuid::Uuid::from_u128(9).into(),
        workload: "ddns".to_owned(),
    };
    let mut tx = pool.begin().await.unwrap();
    let granted = grant(&mut tx, nowhere.clone(), desk.trail(now)).await;
    assert!(matches!(granted, Err(Error::UnknownDevice)), "{granted:?}");
    drop(tx);
    let mut tx = pool.begin().await.unwrap();
    let ended = ungrant(&mut tx, nowhere, desk.trail(now)).await;
    assert!(matches!(ended, Err(Error::NoGrant)), "{ended:?}");
}
