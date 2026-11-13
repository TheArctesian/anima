//! Each kind of channel sends its encoded notification through the one egress path, and the
//! send is logged as an ok delivery, worded by the channel's template when it has one (NOT-002,
//! NOT-004, NOT-007, D-17, P3-003).

use sqlx::PgPool;

use super::{
    DISCORD, NO_CONTENT, answering, apprise, apprise_key, channel, deliveries, dispatcher, email,
    ntfy, queue, row, start, target, webhook,
};
use crate::fleet::notifications::{NotificationKind, egress::tests::servers::Behaviour};

#[sqlx::test(migrations = "./migrations")]
async fn not_002_a_webhook_notification_is_posted_and_logged(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let now = start(&pool).await;
    let queued = queue(&pool, hooks, None, now).await;

    let claimed = dispatcher(&pool, egress).tick(now).await;

    assert_eq!(claimed, 1);
    server.settle().await;
    assert_eq!(
        server.lines(),
        ["POST /hooks/synthetic-path-secret HTTP/1.1"]
    );
    let body: serde_json::Value = serde_json::from_slice(&server.bodies()[0]).unwrap();
    assert_eq!(body["check"], "web");
    assert_eq!(body["kind"], "down");
    assert!(row(&pool, queued).await.sent);
    let logged = deliveries(&pool).await;
    assert_eq!(logged.len(), 1);
    let delivery = &logged[0];
    assert_eq!(
        (
            delivery.channel,
            delivery.channel_name.as_str(),
            delivery.ok
        ),
        (Some(hooks), "hooks", true)
    );
    assert_eq!((delivery.kind, delivery.at), (NotificationKind::Down, now));
    assert_eq!((delivery.reason, delivery.code), (None, None));
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_a_ntfy_notification_is_posted_to_its_topic(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let alerts = channel(&pool, "alerts", ntfy(&server)).await;
    let now = start(&pool).await;
    let queued = queue(&pool, alerts, None, now).await;

    dispatcher(&pool, egress).tick(now).await;

    server.settle().await;
    assert_eq!(server.lines(), ["POST /synthetic-topic-secret HTTP/1.1"]);
    let headers: Vec<String> = server.headers().iter().map(|h| h.to_lowercase()).collect();
    assert!(headers.contains(&"priority: 4".to_owned()), "{headers:?}");
    assert!(
        headers.contains(&"authorization: bearer synthetic-token-secret".to_owned()),
        "{headers:?}"
    );
    // The installation's default body, as plain text.
    let body = String::from_utf8(server.bodies()[0].clone()).unwrap();
    assert!(
        body.starts_with("web on pve-01 was up and is now down"),
        "{body}"
    );
    assert!(body.ends_with("timed out"), "{body}");
    assert!(row(&pool, queued).await.sent);
    assert!(deliveries(&pool).await[0].ok);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_002_an_email_notification_is_mailed(pool: PgPool) {
    let (server, egress) = target(Behaviour::SmtpImplicit).await;
    let mail = channel(&pool, "mail", email(&server)).await;
    let now = start(&pool).await;
    let queued = queue(&pool, mail, None, now).await;

    dispatcher(&pool, egress).tick(now).await;

    server.settle().await;
    let lines = server.lines();
    assert!(
        lines.contains(&"MAIL FROM:<anima@example.test>".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"RCPT TO:<ops@example.test>".to_owned()),
        "{lines:?}"
    );
    assert!(lines.contains(&"DATA".to_owned()), "{lines:?}");
    assert!(row(&pool, queued).await.sent);
    assert!(deliveries(&pool).await[0].ok);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_18_a_stateless_apprise_notification_is_posted_with_its_urls(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let apprise_channel = channel(&pool, "apprise", apprise(&server, &[DISCORD])).await;
    let now = start(&pool).await;
    queue(&pool, apprise_channel, None, now).await;

    dispatcher(&pool, egress).tick(now).await;

    server.settle().await;
    assert_eq!(server.lines(), ["POST /notify HTTP/1.1"]);
    let body: serde_json::Value = serde_json::from_slice(&server.bodies()[0]).unwrap();
    assert_eq!(body["urls"], DISCORD);
    assert!(body.get("attach").is_none() && body.get("attachment").is_none());
    assert!(deliveries(&pool).await[0].ok);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_18_a_config_key_apprise_notification_is_posted_under_its_key(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let keyed = channel(&pool, "keyed", apprise_key(&server)).await;
    let now = start(&pool).await;
    queue(&pool, keyed, None, now).await;

    dispatcher(&pool, egress).tick(now).await;

    server.settle().await;
    assert_eq!(
        server.lines(),
        ["POST /notify/synthetic-key-secret HTTP/1.1"]
    );
    assert!(deliveries(&pool).await[0].ok);
}

#[sqlx::test(migrations = "./migrations")]
async fn p3_003_the_dispatcher_obeys_the_address_policy(pool: PgPool) {
    let (server, _) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let now = start(&pool).await;
    let queued = queue(&pool, hooks, None, now).await;
    // The production policy refuses loopback.
    let strict = crate::fleet::notifications::egress::Egress::new(
        crate::fleet::targets::AddressPolicy::default(),
    );

    dispatcher(&pool, strict).tick(now).await;

    server.settle().await;
    assert_eq!(server.connections(), 0);
    let delivery = &deliveries(&pool).await[0];
    assert_eq!(
        delivery.reason,
        Some(crate::fleet::notifications::DeliveryReason::TargetNotPermitted)
    );
    assert!(row(&pool, queued).await.given_up);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_001_nothing_due_is_nothing_sent(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let now = start(&pool).await;
    queue(&pool, hooks, None, now + chrono::TimeDelta::minutes(1)).await;

    let claimed = dispatcher(&pool, egress).tick(now).await;

    assert_eq!(claimed, 0);
    server.settle().await;
    assert_eq!(server.connections(), 0);
    assert_eq!(deliveries(&pool).await.len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_007_a_channel_template_words_its_notification(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    sqlx::query(
        "UPDATE notifications.channels SET title_template = $2, body_template = $3 WHERE id = $1",
    )
    .bind(hooks)
    .bind("[{status}] {check}")
    .bind("{device}: {message}")
    .execute(&pool)
    .await
    .unwrap();
    let t0 = start(&pool).await;
    queue(&pool, hooks, None, t0).await;

    dispatcher(&pool, egress).tick(t0).await;

    server.settle().await;
    let body: serde_json::Value = serde_json::from_slice(&server.bodies()[0]).unwrap();
    assert_eq!(body["title"], "[down] web");
    assert_eq!(body["body"], "pve-01: timed out");
}
