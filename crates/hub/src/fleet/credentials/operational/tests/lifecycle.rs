//! The versions of a key: one active, a new one supersedes the old at once, a revocation ends
//! it, an expiry ends it, and an ended value goes after 30 days (R-147, K-E5 Section 2).

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{Desk, Metadata, NEXT_VALUE, database_now, metadata, pve, text_of};
use crate::fleet::credentials::operational::{Due, Error, VersionState, forget_ended, key, revoke};

fn states(key: &super::OperationalKey) -> Vec<(i32, VersionState)> {
    key.versions
        .iter()
        .map(|version| (version.number, version.state))
        .collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_a_key_has_one_active_version_and_a_new_one_supersedes_the_old(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    let made = desk.create("cloudflare.dns", metadata(), now).await;
    assert_eq!(made.active_version(), Some(1));
    let first = desk.rotate("cloudflare.dns", NEXT_VALUE, now).await;
    let second = desk.rotate("cloudflare.dns", "third", now).await;

    assert_eq!(first.ended, Some(1));
    assert_eq!(second.ended, Some(2));
    assert_eq!(
        states(&second.key),
        [
            (3, VersionState::Active),
            (2, VersionState::Superseded),
            (1, VersionState::Superseded)
        ]
    );
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM credentials.operational_key_versions WHERE state = 'active'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_a_superseded_version_is_given_no_more(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("cloudflare.dns", metadata(), now).await;
    desk.grant("cloudflare.dns", pve(), now).await;
    let before = desk.fetch(pve(), "cloudflare.dns", now).await.unwrap();
    assert_eq!((before.version, text_of(&before)), (1, super::VALUE));

    desk.rotate("cloudflare.dns", NEXT_VALUE, now).await;

    let after = desk.fetch(pve(), "cloudflare.dns", now).await.unwrap();
    assert_eq!((after.version, text_of(&after)), (2, NEXT_VALUE));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_a_revoked_key_is_given_nothing_and_names_who_still_holds_it(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("cloudflare.dns", metadata(), now).await;
    desk.grant("cloudflare.dns", pve(), now).await;
    let held = [daemon_core::wire::KeyVersion {
        name: "cloudflare.dns".to_owned(),
        version: 1,
    }];
    super::super::record_held(&pool, pve(), &held, now)
        .await
        .unwrap();

    let ending = desk.revoke("cloudflare.dns", now).await;

    assert_eq!(ending.ended, Some(1));
    assert_eq!(ending.key.active_version(), None);
    assert_eq!(states(&ending.key), [(1, VersionState::Revoked)]);
    let holders: Vec<_> = ending.still_held_by.iter().map(|c| c.device).collect();
    assert_eq!(holders, [pve()]);
    assert!(desk.fetch(pve(), "cloudflare.dns", now).await.is_none());
    let current = super::super::current_for(&pool, pve(), now).await.unwrap();
    assert_eq!(current, Vec::new());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_147_a_key_past_its_expiry_reads_expired_and_is_given_no_more(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    let expiring = Metadata {
        expires_at: Some(now + TimeDelta::days(10)),
        ..metadata()
    };
    let made = desk.create("ollama.token", expiring, now).await;
    desk.grant("ollama.token", pve(), now).await;
    assert_eq!(made.due, Due::Expiring);
    assert!(desk.fetch(pve(), "ollama.token", now).await.is_some());

    let later = now + TimeDelta::days(11);
    let read = key(&pool, "ollama.token", later).await.unwrap().unwrap();
    assert_eq!(read.due, Due::Expired);
    assert_eq!(states(&read), [(1, VersionState::Expired)]);
    assert!(desk.fetch(pve(), "ollama.token", later).await.is_none());
    let current = super::super::current_for(&pool, pve(), later)
        .await
        .unwrap();
    assert_eq!(current, Vec::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn r_147_an_ended_value_is_deleted_30_days_after_it_ended(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("cloudflare.dns", metadata(), now).await;
    desk.rotate("cloudflare.dns", NEXT_VALUE, now).await;
    let sealed_lengths = || async {
        sqlx::query_scalar::<_, i32>(
            "SELECT length(sealed) FROM credentials.operational_key_versions ORDER BY number",
        )
        .fetch_all(&pool)
        .await
        .unwrap()
    };

    assert_eq!(
        forget_ended(&pool, now + TimeDelta::days(29))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        forget_ended(&pool, now + TimeDelta::days(31))
            .await
            .unwrap(),
        1
    );

    let lengths = sealed_lengths().await;
    assert_eq!(lengths[0], 0, "the superseded value is gone");
    assert!(lengths[1] > 0, "the active value stays");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_147_a_version_that_expired_stays_ended_when_the_expiry_moves_later(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    let short = Metadata {
        expires_at: Some(now + TimeDelta::days(1)),
        ..metadata()
    };
    desk.create("ollama.token", short, now).await;
    let later = now + TimeDelta::days(2);
    let longer = Metadata {
        expires_at: Some(later + TimeDelta::days(100)),
        ..metadata()
    };
    let mut tx = pool.begin().await.unwrap();
    let changed = super::super::update(&mut tx, "ollama.token", longer, desk.trail(later))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(states(&changed), [(1, VersionState::Expired)]);
    assert_eq!(changed.active_version(), None);
}

#[sqlx::test(migrations = "./migrations")]
async fn r_147_a_key_past_its_expiry_takes_no_new_value_until_the_expiry_moves(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    let expiring = Metadata {
        expires_at: Some(now + TimeDelta::days(1)),
        ..metadata()
    };
    desk.create("ollama.token", expiring, now).await;
    let later = now + TimeDelta::days(2);

    let refused = desk.try_rotate("ollama.token", NEXT_VALUE, later).await;

    assert!(matches!(refused, Err(Error::Expired)));
    let read = key(&pool, "ollama.token", later).await.unwrap().unwrap();
    assert_eq!(states(&read), [(1, VersionState::Expired)], "no version 2");
}

#[sqlx::test(migrations = "./migrations")]
async fn r_147_a_revoke_with_no_active_version_is_refused_and_writes_nothing(pool: PgPool) {
    let now = database_now(&pool).await;
    let desk = Desk::new(&pool);
    desk.create("ollama.token", metadata(), now).await;
    desk.revoke("ollama.token", now).await;
    let entries = audit_entries(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    let again = revoke(&mut tx, "ollama.token", desk.trail(now)).await;

    assert!(matches!(again, Err(Error::NoActiveVersion)));
    drop(tx);
    assert_eq!(audit_entries(&pool).await, entries);
}

async fn audit_entries(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM events.audit_log")
        .fetch_one(pool)
        .await
        .unwrap()
}
