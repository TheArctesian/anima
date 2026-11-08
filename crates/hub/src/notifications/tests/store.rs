//! What the store reads back: channels, check state, deliveries and settings.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{PING, SSH_PORT, channel, check, fields, now};
use crate::{
    ids::{ChannelId, DeliveryId},
    notifications::{
        ChannelKind, CheckState, Delivery, DeliveryCursor, ExpiryMark, NotificationKind,
        store as sql,
    },
    status::Status,
};

fn delivery(channel: ChannelId, at: DateTime<Utc>, ok: bool) -> Delivery {
    Delivery {
        id: DeliveryId::from(Uuid::now_v7()),
        channel: Some(channel),
        channel_name: "ops".to_owned(),
        kind: NotificationKind::Up,
        at,
        ok,
        message: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_a_channel_is_stored_updated_and_listed_with_its_sealed_secrets(pool: PgPool) {
    let id = channel(&pool, "b-ops").await;
    let other = channel(&pool, "a-ops").await;
    let mut conn = pool.acquire().await.unwrap();
    let mut changed = fields("b-ops");
    changed.kind = ChannelKind::Email;
    changed.secrets = None;
    changed.body_template = Some("{check} is {status}".to_owned());
    changed.on_by_default = true;

    assert!(
        sql::update_channel(&mut conn, id, &changed, now(&pool).await)
            .await
            .unwrap()
    );

    let all = sql::list_channels(&pool, None).await.unwrap();
    assert_eq!(all.iter().map(|c| c.id).collect::<Vec<_>>(), [other, id]);
    assert!(all[1].fields == changed);
    assert_eq!(all[0].fields.secrets, Some(vec![1, 2, 3]));
    let one = sql::list_channels(&pool, Some(&[id])).await.unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(sql::on_by_default(&pool).await.unwrap(), [id]);
    let missing = ChannelId::from(Uuid::now_v7());
    assert!(
        !sql::update_channel(&mut conn, missing, &changed, now(&pool).await)
            .await
            .unwrap()
    );
    assert!(!sql::delete_channel(&mut conn, missing).await.unwrap());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_005_check_state_starts_empty_and_is_written_back_in_the_transaction(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    let first = sql::lock_check_state(&mut tx, check(PING)).await.unwrap();
    let marks = vec![ExpiryMark {
        threshold_days: 14,
        not_after: DateTime::from_timestamp(1_900_000_000, 0).unwrap(),
    }];
    let state = CheckState {
        notified_status: Some(Status::Down),
        down_beats: 4,
        expiry_marks: marks,
    };
    sql::write_check_state(&mut tx, check(PING), &state)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let after_rollback = sql::lock_check_state(&mut pool.acquire().await.unwrap(), check(PING))
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sql::write_check_state(&mut tx, check(PING), &state)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let stored = sql::lock_check_state(&mut conn, check(PING)).await.unwrap();

    assert_eq!(first, CheckState::default());
    assert_eq!(after_rollback, CheckState::default());
    assert_eq!(stored, state);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_003_channels_and_settings_are_read_for_many_checks_at_once(pool: PgPool) {
    let a = channel(&pool, "a").await;
    let b = channel(&pool, "b").await;
    let mut conn = pool.acquire().await.unwrap();
    sql::set_check_channels(&mut conn, check(PING), &[b, a])
        .await
        .unwrap();
    sql::set_check_channels(&mut conn, check(PING), &[a])
        .await
        .unwrap();
    sql::set_check_channels(&mut conn, check(SSH_PORT), &[a, b])
        .await
        .unwrap();
    sql::write_check_settings(
        &mut conn,
        check(PING),
        crate::notifications::CheckSettings {
            resend_every_beats: 12,
        },
    )
    .await
    .unwrap();

    let links = sql::check_channels(&pool, &[check(PING), check(SSH_PORT)])
        .await
        .unwrap();
    let settings = sql::check_settings(&pool, &[check(PING), check(SSH_PORT)])
        .await
        .unwrap();

    assert_eq!(links[&check(PING)], [a]);
    assert_eq!(links[&check(SSH_PORT)], [a, b]);
    assert_eq!(settings[&check(PING)].resend_every_beats, 12);
    assert!(!settings.contains_key(&check(SSH_PORT)));
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_deliveries_page_newest_first_by_keyset_and_last_per_channel(pool: PgPool) {
    let ops = channel(&pool, "ops").await;
    let other = channel(&pool, "other").await;
    let at = now(&pool).await;
    let mut rows = Vec::new();
    for minutes in 0..5 {
        rows.push(delivery(
            ops,
            at - TimeDelta::minutes(minutes),
            minutes % 2 == 0,
        ));
    }
    rows.push(delivery(other, at - TimeDelta::minutes(10), true));
    for row in &rows {
        sql::insert_delivery(&pool, row).await.unwrap();
    }

    let first = sql::deliveries_page(&pool, None, None, 3).await.unwrap();
    let cursor = DeliveryCursor {
        at: first[2].at,
        id: first[2].id,
    };
    let second = sql::deliveries_page(&pool, None, Some(cursor), 3)
        .await
        .unwrap();
    let only_other = sql::deliveries_page(&pool, Some(other), None, 10)
        .await
        .unwrap();
    let last = sql::last_deliveries(&pool, &[ops, other]).await.unwrap();

    assert_eq!(first, rows[..3]);
    assert_eq!(second, [rows[3].clone(), rows[4].clone(), rows[5].clone()]);
    assert_eq!(only_other, [rows[5].clone()]);
    assert_eq!(last[&ops], rows[0]);
    assert_eq!(last[&other], rows[5]);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_007_installation_settings_round_trip(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let mut settings = sql::installation_settings(&mut *conn).await.unwrap();
    settings.certificate_days = vec![30, 10];
    settings.title_template = "{check}".to_owned();

    sql::write_installation_settings(&mut conn, &settings)
        .await
        .unwrap();

    assert_eq!(sql::installation_settings(&pool).await.unwrap(), settings);
}
