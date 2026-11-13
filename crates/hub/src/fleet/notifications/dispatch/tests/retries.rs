//! Failed sends: the retry schedule, giving up with one Warning event, permanent failures, an
//! unresolved host that is retried, channels turned off and channels that cannot be read (D-16,
//! R-103, NOT-001, NOT-002, NOT-004).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    NO_CONTENT, answering, channel, deliveries, dispatcher, queue, row, secret, start, vault,
    warnings, webhook,
};
use crate::{
    fleet::notifications::{
        ChannelFields, ChannelKind, DeliveryReason,
        channels::{Secrets, Settings},
        sealing, store,
    },
    ids::{ChannelId, CheckId},
};

const UNAVAILABLE: &[u8] = b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n";
const NOT_FOUND: &[u8] = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";

/// A fixture check (K6): the first one on the first fixture device.
const PING: &str = "0190a000-0000-7000-8003-000000000001";

fn minutes(count: i64) -> TimeDelta {
    TimeDelta::minutes(count)
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_failing_target_is_tried_after_1_5_15_and_60_minutes_then_given_up_once(
    pool: PgPool,
) {
    let (server, egress) = answering(UNAVAILABLE).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    let dispatcher = dispatcher(&pool, egress);
    let mut tries: Vec<DateTime<Utc>> = Vec::new();

    let mut now = t0;
    for wait in [0, 1, 5, 15, 60] {
        now += minutes(wait);
        assert_eq!(
            dispatcher.tick(now - TimeDelta::seconds(1)).await,
            0,
            "early"
        );
        assert_eq!(dispatcher.tick(now).await, 1, "due");
        tries.push(now);
    }
    let later = dispatcher.tick(now + TimeDelta::days(1)).await;

    assert_eq!(later, 0);
    server.settle().await;
    assert_eq!(server.connections(), 5);
    let state = row(&pool, queued).await;
    assert_eq!(
        (state.attempts, state.sent, state.given_up),
        (5, false, true)
    );
    let logged = deliveries(&pool).await;
    assert_eq!(logged.iter().map(|d| d.at).collect::<Vec<_>>(), tries);
    assert!(
        logged
            .iter()
            .all(|d| !d.ok && d.reason == Some(DeliveryReason::HttpStatus) && d.code == Some(503))
    );
    assert_eq!(
        warnings(&pool).await,
        [
            "Notification channel hooks gave up a Down notification for web on pve-01 after 5 \
          tries: http_status 503."
        ]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_retried_send_that_succeeds_is_sent_with_no_warning(pool: PgPool) {
    let (failing, egress) = answering(UNAVAILABLE).await;
    let hooks = channel(&pool, "hooks", webhook(&failing)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    dispatcher(&pool, egress).tick(t0).await;
    // The target recovers: the channel now points at a server that answers.
    let (working, egress) = answering(NO_CONTENT).await;
    move_channel(&pool, hooks, webhook(&working)).await;

    dispatcher(&pool, egress).tick(t0 + minutes(1)).await;

    let state = row(&pool, queued).await;
    assert_eq!(
        (state.attempts, state.sent, state.given_up),
        (2, true, false)
    );
    let oks: Vec<bool> = deliveries(&pool).await.iter().map(|d| d.ok).collect();
    assert_eq!(oks, [false, true]);
    assert_eq!(warnings(&pool).await.len(), 0);
}

/// Points `id` at new settings, its secrets sealed for them, as an update would.
async fn move_channel(
    pool: &PgPool,
    id: ChannelId,
    (settings, secrets): (
        crate::fleet::notifications::channels::Settings,
        crate::fleet::notifications::channels::Secrets,
    ),
) {
    let sealed = sealing::seal(&vault(), id, &settings, &secrets).unwrap();
    sqlx::query("UPDATE notifications.channels SET settings = $2, secrets = $3 WHERE id = $1")
        .bind(id)
        .bind(settings.to_stored())
        .bind(sealed)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_permanent_failure_is_given_up_at_once_with_one_warning(pool: PgPool) {
    let (server, egress) = answering(NOT_FOUND).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    let dispatcher = dispatcher(&pool, egress);

    dispatcher.tick(t0).await;
    let again = dispatcher.tick(t0 + TimeDelta::days(1)).await;

    assert_eq!(again, 0);
    server.settle().await;
    assert_eq!(server.connections(), 1);
    let state = row(&pool, queued).await;
    assert_eq!((state.attempts, state.given_up), (1, true));
    assert_eq!(deliveries(&pool).await[0].code, Some(404));
    assert_eq!(
        warnings(&pool).await,
        [
            "Notification channel hooks gave up a Down notification for web on pve-01 after 1 \
          try: http_status 404."
        ]
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_003_a_channel_turned_off_for_the_check_sends_nothing(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let ping: CheckId = PING.parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    store::set_check_channels(&mut conn, ping, &[hooks])
        .await
        .unwrap();
    store::lock_check_state(&mut conn, ping).await.unwrap();
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, Some(ping), t0).await;
    store::set_check_channels(&mut conn, ping, &[])
        .await
        .unwrap();
    drop(conn);

    let claimed = dispatcher(&pool, egress).tick(t0).await;

    assert_eq!(claimed, 1);
    server.settle().await;
    assert_eq!(server.connections(), 0);
    assert_eq!(deliveries(&pool).await.len(), 0);
    assert_eq!(warnings(&pool).await.len(), 0);
    let state = row(&pool, queued).await;
    assert_eq!((state.sent, state.given_up), (false, true));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices", "checks"))
)]
async fn not_003_a_channel_still_on_for_the_check_sends(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let ping: CheckId = PING.parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    store::set_check_channels(&mut conn, ping, &[hooks])
        .await
        .unwrap();
    store::lock_check_state(&mut conn, ping).await.unwrap();
    drop(conn);
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, Some(ping), t0).await;

    dispatcher(&pool, egress).tick(t0).await;

    assert!(row(&pool, queued).await.sent);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_001_a_notification_about_a_deleted_check_is_still_sent(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let gone = CheckId::from(Uuid::now_v7());
    let queued = queue(&pool, hooks, Some(gone), t0).await;

    dispatcher(&pool, egress).tick(t0).await;

    assert!(row(&pool, queued).await.sent);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_a_channel_deleted_after_the_claim_ends_the_row_unsent(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    queue(&pool, hooks, None, t0).await;
    let claimed = store::claim_due(&pool, t0, minutes(5), 1).await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    store::delete_channel(&mut conn, hooks).await.unwrap();
    drop(conn);
    let defaults = store::installation_settings(&pool).await.unwrap();

    let finished = dispatcher(&pool, egress)
        .finish(&claimed[0], &defaults, t0)
        .await
        .unwrap();

    assert_eq!(finished, super::super::claimed::Finished::Ended);
    server.settle().await;
    assert_eq!(server.connections(), 0);
    assert_eq!(deliveries(&pool).await.len(), 0);
}

/// Stores a channel whose stored settings this version cannot read.
async fn unreadable_channel(pool: &PgPool) -> ChannelId {
    let id = ChannelId::from(Uuid::now_v7());
    let fields = ChannelFields {
        name: "broken".to_owned(),
        kind: ChannelKind::Ntfy,
        on_by_default: false,
        settings: serde_json::json!({ "server": 5 }),
        secrets: None,
        title_template: None,
        body_template: None,
    };
    let mut conn = pool.acquire().await.unwrap();
    store::insert_channel(&mut conn, id, &fields, Utc::now())
        .await
        .unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_a_channel_whose_settings_cannot_be_read_gets_a_delivery_and_sends_nothing(
    pool: PgPool,
) {
    let (server, egress) = answering(NO_CONTENT).await;
    let broken = unreadable_channel(&pool).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, broken, None, t0).await;

    dispatcher(&pool, egress).tick(t0).await;

    server.settle().await;
    assert_eq!(server.connections(), 0);
    let logged = deliveries(&pool).await;
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].reason, Some(DeliveryReason::SetupFailed));
    assert!(row(&pool, queued).await.given_up);
    assert_eq!(warnings(&pool).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_005_secrets_sealed_for_another_channel_are_secret_unreadable(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let (settings, secrets) = webhook(&server);
    let foreign = sealing::seal(
        &vault(),
        ChannelId::from(Uuid::now_v7()),
        &settings,
        &secrets,
    )
    .unwrap();
    sqlx::query("UPDATE notifications.channels SET secrets = $2 WHERE id = $1")
        .bind(hooks)
        .bind(foreign)
        .execute(&pool)
        .await
        .unwrap();
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;

    dispatcher(&pool, egress).tick(t0).await;

    server.settle().await;
    assert_eq!(server.connections(), 0);
    assert_eq!(
        deliveries(&pool).await[0].reason,
        Some(DeliveryReason::SecretUnreadable)
    );
    assert!(row(&pool, queued).await.given_up);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_r_103_a_host_that_does_not_resolve_is_tried_again_not_given_up(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let (Settings::Webhook(mut settings), Secrets::Webhook(mut secrets)) = webhook(&server) else {
        unreachable!("a webhook fixture");
    };
    // `.invalid` never resolves (RFC 6761).
    settings.origin = "https://nothing-here.invalid".to_owned();
    secrets.url = secret("https://nothing-here.invalid/hooks/synthetic-path-secret");
    let fields = (Settings::Webhook(settings), Secrets::Webhook(secrets));
    let hooks = channel(&pool, "hooks", fields).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;

    dispatcher(&pool, egress).tick(t0).await;

    let state = row(&pool, queued).await;
    assert_eq!(
        (state.attempts, state.given_up, state.next_attempt_at),
        (1, false, t0 + minutes(1))
    );
    assert_eq!(
        deliveries(&pool).await[0].reason,
        Some(DeliveryReason::TargetNotPermitted)
    );
    assert_eq!(warnings(&pool).await.len(), 0);
}
