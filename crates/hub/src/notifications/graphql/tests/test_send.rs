//! `sendTestNotification`: sent at once on the dispatcher's path, within the time limit,
//! logged as a delivery with one audit entry, never queued (NOT-004, LOG-005, LOG-007).

use std::{net::IpAddr, time::Duration};

use chrono::Utc;
use sqlx::PgPool;
use tokio::time::Instant;
use uuid::Uuid;

use crate::{
    events::{Bus, ClientAddr, Operator},
    ids::{ChannelId, OperatorId},
    notifications::{
        Delivery, DeliveryReason, NotificationKind,
        dispatch::{
            Sender,
            tests::{
                NO_CONTENT, assert_no_secret, channel, recorded, start, target, vault, webhook,
            },
        },
        egress::{
            Egress, TIME_LIMIT,
            tests::servers::{Behaviour, Server},
        },
        graphql::test_send::{Asker, send_test},
    },
};

/// How late past the time limit a send may end on a loaded machine.
const SLACK: Duration = Duration::from_secs(3);

fn operator() -> Operator {
    Operator {
        id: OperatorId::from(Uuid::now_v7()),
        name: "operator".to_owned(),
    }
}

fn asker() -> Asker {
    let client = ClientAddr("192.0.2.10".parse::<IpAddr>().unwrap());
    (Bus::new(16), (operator(), client), Utc::now())
}

async fn send(pool: &PgPool, egress: Egress, id: ChannelId) -> async_graphql::Result<Delivery> {
    send_test(pool, &Sender::new(vault(), egress), asker(), id).await
}

async fn answering_hook(pool: &PgPool, reply: &[u8]) -> (Server, Egress, ChannelId) {
    let reply = reply.to_vec();
    let (server, egress) = target(|acceptor| Behaviour::Http(reply, Some(acceptor))).await;
    let id = channel(pool, "hooks", webhook(&server)).await;
    (server, egress, id)
}

async fn outbox_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM notifications.outbox")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_a_test_notification_is_sent_at_once_and_logged(pool: PgPool) {
    start(&pool).await;
    let (server, egress, hooks) = answering_hook(&pool, NO_CONTENT).await;

    let delivery = send(&pool, egress, hooks).await.unwrap();

    assert!(delivery.ok);
    assert_eq!(
        (delivery.kind, delivery.channel, delivery.reason),
        (NotificationKind::Test, Some(hooks), None)
    );
    server.settle().await;
    let body: serde_json::Value = serde_json::from_slice(&server.bodies()[0]).unwrap();
    assert_eq!(body["kind"], "test");
    assert_eq!(body["check"], "Test notification");
    assert_eq!(body["device"], "Anima");
    assert_eq!(outbox_rows(&pool).await, 0, "a test is never queued");
    let stored = crate::notifications::store::deliveries_page(&pool, Some(hooks), None, 10)
        .await
        .unwrap();
    assert_eq!(stored, [delivery]);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_test_notification_is_audited_once_without_a_secret(pool: PgPool) {
    start(&pool).await;
    let (_server, egress, hooks) = answering_hook(&pool, NO_CONTENT).await;

    send(&pool, egress, hooks).await.unwrap();

    let audits: Vec<(String, String, String, serde_json::Value)> =
        sqlx::query_as("SELECT action, target_type, target_id, change FROM events.audit_log")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(audits.len(), 1);
    let (action, target_type, target_id, change) = &audits[0];
    assert_eq!((action.as_str(), target_type.as_str()), ("send", "channel"));
    assert_eq!(target_id, &hooks.to_string());
    assert_eq!(
        change,
        &serde_json::json!({ "name": "hooks", "ok": true, "reason": null, "code": null })
    );
    let events: Vec<String> = sqlx::query_scalar("SELECT message FROM events.event_log")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        events,
        ["Notification channel hooks sent a test notification."]
    );
    assert_no_secret(&recorded(&pool).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_a_failed_test_is_answered_as_a_delivery_with_its_reason(pool: PgPool) {
    start(&pool).await;
    let (_server, egress, hooks) = answering_hook(
        &pool,
        b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n",
    )
    .await;

    let delivery = send(&pool, egress, hooks).await.unwrap();

    assert!(!delivery.ok);
    assert_eq!(
        (delivery.reason, delivery.code),
        (Some(DeliveryReason::HttpStatus), Some(401))
    );
    let events: Vec<String> = sqlx::query_scalar("SELECT message FROM events.event_log")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        events,
        ["Notification channel hooks failed to send a test notification: http_status."]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_a_test_to_a_silent_target_ends_within_the_time_limit(pool: PgPool) {
    start(&pool).await;
    let (server, egress) = target(|_| Behaviour::Silent).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let started = Instant::now();

    let delivery = send(&pool, egress, hooks).await.unwrap();

    assert!(
        started.elapsed() < TIME_LIMIT + SLACK,
        "{:?}",
        started.elapsed()
    );
    assert_eq!(delivery.reason, Some(DeliveryReason::TimedOut));
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_a_test_through_no_such_channel_is_not_found_and_records_nothing(pool: PgPool) {
    start(&pool).await;
    let (_server, egress, _) = answering_hook(&pool, NO_CONTENT).await;

    let error = send(&pool, egress, ChannelId::from(Uuid::now_v7()))
        .await
        .unwrap_err();

    assert_eq!(error.message, "No such channel.");
    assert_eq!(recorded(&pool).await, "");
    let deliveries = crate::notifications::store::deliveries_page(&pool, None, None, 10)
        .await
        .unwrap();
    assert_eq!(deliveries.len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_test_whose_request_is_dropped_after_the_target_has_it_is_still_recorded(
    pool: PgPool,
) {
    start(&pool).await;
    let (server, egress) = target(|acceptor| {
        Behaviour::Stalled(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\n",
            acceptor,
        )
    })
    .await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;

    // The client goes away while the hub still reads the reply body.
    let dropped = tokio::time::timeout(Duration::from_secs(2), send(&pool, egress, hooks)).await;

    assert!(dropped.is_err(), "the send was still reading the reply");
    let audited = async {
        while count(&pool, "events.audit_log").await == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    tokio::time::timeout(TIME_LIMIT + SLACK, audited)
        .await
        .expect("the dropped test send is recorded");
    assert_eq!(count(&pool, "events.audit_log").await, 1);
    assert_eq!(count(&pool, "events.event_log").await, 1);
    let deliveries = crate::notifications::store::deliveries_page(&pool, None, None, 10)
        .await
        .unwrap();
    assert_eq!(
        deliveries.iter().map(|d| d.ok).collect::<Vec<_>>(),
        [true],
        "the 2xx head decided"
    );
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}
