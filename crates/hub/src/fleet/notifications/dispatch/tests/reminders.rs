//! A reminder goes through the same outbox, dispatcher and channels as a status notification
//! (REM-008, REM-010; K-E6 Section 5).

use sqlx::PgPool;
use uuid::Uuid;

use super::{
    NO_CONTENT, answering, channel, deliveries, dispatcher, ntfy, origin, secret, start, vault,
    webhook,
};
use crate::fleet::notifications::{
    NotificationKind, Queued, Reminder,
    channels::{NtfySecrets, NtfySettings, Secrets, Settings},
    queue_reminder,
};

fn reminder() -> Reminder {
    Reminder {
        firing: Uuid::from_u128(7),
        title: "Train".to_owned(),
        body: "Time to train: {device} run".to_owned(),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_010_a_reminder_is_sent_through_the_channels_with_its_own_words(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let alerts = channel(&pool, "alerts", ntfy(&server)).await;
    let now = start(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    let queued = queue_reminder(&mut tx, &vault(), &[hooks, alerts], (&reminder(), now))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        queued,
        Queued {
            rows: 2,
            topic_not_random: 0
        }
    );

    assert_eq!(dispatcher(&pool, egress).tick(now).await, 2);
    server.settle().await;
    let bodies: Vec<String> = server
        .bodies()
        .iter()
        .map(|body| String::from_utf8_lossy(body).into_owned())
        .collect();
    let posted = bodies.iter().find(|body| body.starts_with('{')).unwrap();
    let posted: serde_json::Value = serde_json::from_str(posted).unwrap();
    assert_eq!(posted["kind"], "reminder");
    assert_eq!(posted["title"], "Train");
    assert_eq!(
        posted["body"], "Time to train: {device} run",
        "the text of a rule is never read as a template"
    );
    assert!(posted["check"].is_null());
    assert!(
        bodies
            .iter()
            .any(|body| body == "Time to train: {device} run"),
        "{bodies:?}"
    );
    let logged = deliveries(&pool).await;
    assert_eq!(logged.len(), 2);
    assert!(
        logged
            .iter()
            .all(|d| d.ok && d.kind == NotificationKind::Reminder)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_008_an_ntfy_channel_with_a_short_topic_gets_no_reminder(pool: PgPool) {
    let (server, _egress) = answering(NO_CONTENT).await;
    let settings = Settings::Ntfy(NtfySettings {
        server: origin(&server),
        priority: 3,
        token_stored: false,
    });
    let guessable = Secrets::Ntfy(NtfySecrets {
        topic: secret("home-alerts"),
        token: None,
    });
    let short = channel(&pool, "short", (settings, guessable)).await;
    let random = channel(&pool, "random", ntfy(&server)).await;
    let now = start(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    let queued = queue_reminder(&mut conn, &vault(), &[short, random], (&reminder(), now))
        .await
        .unwrap();
    assert_eq!(
        queued,
        Queued {
            rows: 1,
            topic_not_random: 1
        }
    );
    let channels: Vec<Uuid> = sqlx::query_scalar("select channel_id from notifications.outbox")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(channels, [Uuid::from(random)]);
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_008_a_rule_with_no_channels_uses_each_channel_that_is_on_by_default(pool: PgPool) {
    let (server, _egress) = answering(NO_CONTENT).await;
    let chosen = channel(&pool, "chosen", webhook(&server)).await;
    channel(&pool, "other", webhook(&server)).await;
    sqlx::query("update notifications.channels set on_by_default = true where id = $1")
        .bind(Uuid::from(chosen))
        .execute(&pool)
        .await
        .unwrap();
    let now = start(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    let queued = queue_reminder(&mut conn, &vault(), &[], (&reminder(), now))
        .await
        .unwrap();
    assert_eq!(queued.rows, 1);
    let channels: Vec<Uuid> = sqlx::query_scalar("select channel_id from notifications.outbox")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(channels, [Uuid::from(chosen)]);
}
