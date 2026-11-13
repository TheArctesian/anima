//! `observe`, `adopt_defaults` and `for_checks` against a real database (D-16, NOT-001,
//! NOT-003, NOT-005, API-005).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{PING, SSH_PORT, channel, check, count, fields, now};
use crate::{
    fleet::notifications::{
        CheckSettings, Observation, adopt_defaults, for_checks, observe, store as sql,
    },
    ids::{CheckId, DeviceId},
    status::Status::{self, Down, Up},
};

fn beat(check: CheckId, status: Status, at: DateTime<Utc>) -> Observation {
    Observation {
        check,
        check_name: "ping\u{202e}".into(),
        check_type: "ping",
        device: (DeviceId::from(uuid::Uuid::nil()), "pve-01".into()),
        site: "home".into(),
        previous: Up,
        status,
        in_maintenance: false,
        at,
        message: Some("timed out\n".into()),
        certificate_not_after: None,
        domain_not_after: None,
    }
}

async fn observe_in_tx(pool: &PgPool, o: &Observation) {
    let mut tx = pool.begin().await.unwrap();
    observe(&mut tx, o).await.unwrap();
    tx.commit().await.unwrap();
}

async fn link(pool: &PgPool, check: CheckId, channels: &[crate::ids::ChannelId]) {
    let mut conn = pool.acquire().await.unwrap();
    sql::set_check_channels(&mut conn, check, channels)
        .await
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_001_observe_queues_one_row_per_enabled_channel_with_cleaned_placeholders(
    pool: PgPool,
) {
    let a = channel(&pool, "a").await;
    let b = channel(&pool, "b").await;
    let _unlinked = channel(&pool, "c").await;
    link(&pool, check(PING), &[a, b]).await;
    let at = now(&pool).await;

    observe_in_tx(&pool, &beat(check(PING), Down, at)).await;

    assert_eq!(count(&pool, "outbox").await, 2);
    let (kind, payload): (String, serde_json::Value) =
        sqlx::query_as("SELECT kind, payload FROM notifications.outbox LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kind, "down");
    assert_eq!(payload["device"], "pve-01");
    assert_eq!(payload["check"], "ping");
    assert_eq!(payload["status"], "down");
    assert_eq!(payload["previous"], "up");
    assert_eq!(payload["message"], "timed out");
    assert!(payload["days"].is_null());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_001_a_rollback_leaves_neither_rows_nor_state(pool: PgPool) {
    let a = channel(&pool, "a").await;
    link(&pool, check(PING), &[a]).await;
    let at = now(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    observe(&mut tx, &beat(check(PING), Down, at))
        .await
        .unwrap();
    tx.rollback().await.unwrap();

    assert_eq!(count(&pool, "outbox").await, 0);
    let state = sql::lock_check_state(&mut pool.acquire().await.unwrap(), check(PING))
        .await
        .unwrap();
    assert_eq!(state.notified_status, None);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_001_the_same_down_sequence_sends_down_once_and_up_once(pool: PgPool) {
    let a = channel(&pool, "a").await;
    link(&pool, check(PING), &[a]).await;
    let at = now(&pool).await;
    for (i, status) in (0..).zip([Down, Down, Down, Up, Up]) {
        let at = at + TimeDelta::seconds(i);
        observe_in_tx(&pool, &beat(check(PING), status, at)).await;
    }
    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM notifications.outbox ORDER BY created_at")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(kinds, ["down", "up"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_001_no_channels_still_updates_state_and_queues_nothing(pool: PgPool) {
    observe_in_tx(&pool, &beat(check(PING), Down, now(&pool).await)).await;
    assert_eq!(count(&pool, "outbox").await, 0);
    let state = sql::lock_check_state(&mut pool.acquire().await.unwrap(), check(PING))
        .await
        .unwrap();
    assert_eq!(state.notified_status, Some(Down));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_005_observe_reads_the_checks_resend_setting(pool: PgPool) {
    let a = channel(&pool, "a").await;
    link(&pool, check(PING), &[a]).await;
    let mut conn = pool.acquire().await.unwrap();
    sql::write_check_settings(
        &mut conn,
        check(PING),
        CheckSettings {
            resend_every_beats: 2,
        },
    )
    .await
    .unwrap();
    drop(conn);
    let at = now(&pool).await;
    for i in 0..3 {
        observe_in_tx(&pool, &beat(check(PING), Down, at + TimeDelta::seconds(i))).await;
    }
    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM notifications.outbox ORDER BY created_at")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(kinds, ["down", "still_down"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_006_observe_queues_an_expiry_with_days_and_marks_it_once(pool: PgPool) {
    let a = channel(&pool, "a").await;
    link(&pool, check(PING), &[a]).await;
    let at = now(&pool).await;
    for i in 0..2 {
        let mut o = beat(check(PING), Up, at + TimeDelta::seconds(i));
        o.certificate_not_after = Some(at + TimeDelta::days(5));
        observe_in_tx(&pool, &o).await;
    }
    let rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT kind, payload FROM notifications.outbox")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "certificate_expiry");
    assert_eq!(rows[0].1["days"], 5);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_003_adopt_defaults_links_only_channels_on_by_default(pool: PgPool) {
    let mut on = fields("on");
    on.on_by_default = true;
    let on_id = crate::ids::ChannelId::from(uuid::Uuid::now_v7());
    let mut conn = pool.acquire().await.unwrap();
    sql::insert_channel(&mut conn, on_id, &on, now(&pool).await)
        .await
        .unwrap();
    let _off = channel(&pool, "off").await;

    let mut tx = pool.begin().await.unwrap();
    adopt_defaults(&mut tx, check(PING)).await.unwrap();
    tx.commit().await.unwrap();

    let linked = sql::check_channels(&pool, &[check(PING), check(SSH_PORT)])
        .await
        .unwrap();
    assert_eq!(linked.get(&check(PING)), Some(&vec![on_id]));
    assert!(!linked.contains_key(&check(SSH_PORT)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn api_005_for_checks_batches_channels_and_defaults_without_secrets(pool: PgPool) {
    let a = channel(&pool, "a").await;
    let b = channel(&pool, "b").await;
    link(&pool, check(PING), &[b, a]).await;
    let mut conn = pool.acquire().await.unwrap();
    sql::write_check_settings(
        &mut conn,
        check(PING),
        CheckSettings {
            resend_every_beats: 5,
        },
    )
    .await
    .unwrap();
    drop(conn);

    let got = for_checks(&pool, &[check(PING), check(SSH_PORT)])
        .await
        .unwrap();

    let ping = &got[&check(PING)];
    let names: Vec<_> = ping
        .channels
        .iter()
        .map(|c| c.fields.name.as_str())
        .collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(ping.resend_every_beats, 5);
    assert!(ping.channels.iter().all(|c| c.fields.secrets.is_none()));
    let ssh = &got[&check(SSH_PORT)];
    assert!(ssh.channels.is_empty());
    assert_eq!(ssh.resend_every_beats, 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn log_007_a_dirty_device_name_is_cleaned_and_a_201_character_check_name_is_cut_at_200(
    pool: PgPool,
) {
    let a = channel(&pool, "a").await;
    link(&pool, check(PING), &[a]).await;
    let mut o = beat(check(PING), Down, now(&pool).await);
    o.device.1 = "pve\u{202e}-\u{7}01\r\n\u{200b}".into();
    o.check_name = format!("{}x", "c".repeat(200));

    observe_in_tx(&pool, &o).await;

    let payload: serde_json::Value = sqlx::query_scalar("SELECT payload FROM notifications.outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(payload["device"], "pve-01");
    assert_eq!(payload["check"], "c".repeat(200));
}
