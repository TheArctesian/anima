//! The sender against the database: the check at send time, the words for a lock screen and
//! the outbox of the notifications (REM-005, REM-008, REM-009, REM-010).

use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::fleet::{events::Bus, notifications::ChannelKind};

fn sending(pool: &PgPool) -> Sending {
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(
        &mut key_file,
        b"BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=\n",
    )
    .unwrap();
    Sending {
        db: pool.clone(),
        bus: Bus::new(16),
        vault: crate::fleet::credentials::Vault::from_key_file(key_file.path()).unwrap(),
        live: Live::default(),
    }
}

/// A webhook channel that is on by default. Its settings are not read before the dispatcher
/// sends, so the row is enough for the outbox.
async fn default_channel(pool: &PgPool) {
    sqlx::query(
        "insert into notifications.channels (id, name, kind, on_by_default, settings) \
         values ($1, 'hooks', $2, true, '{}')",
    )
    .bind(Uuid::now_v7())
    .bind(ChannelKind::Webhook.as_str())
    .execute(pool)
    .await
    .unwrap();
}

async fn noticed(pool: &PgPool, new: &[Occurrence], now: &str) {
    let mut tx = pool.begin().await.unwrap();
    notice(&mut tx, new, at(now)).await.unwrap();
    tx.commit().await.unwrap();
}

async fn outbox(pool: &PgPool) -> Vec<(String, serde_json::Value)> {
    sqlx::query_as("select kind, payload from notifications.outbox order by created_at")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[test]
fn rem_009_the_words_for_a_lock_screen_hold_no_private_value() {
    assert_eq!(
        lock_screen(Class::C, "Backups", "The backup of pve-01 is late"),
        (
            "Backups".to_owned(),
            "The backup of pve-01 is late".to_owned()
        )
    );
    for class in [Class::A, Class::B] {
        let (title, body) = lock_screen(class, "Passport", "Passport ends in 7 days");
        assert_eq!(
            (title.as_str(), body.as_str()),
            ("Anima", "You have a reminder.")
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_010_a_firing_is_queued_one_time_with_the_words_of_its_class(pool: PgPool) {
    default_channel(&pool).await;
    add(&pool, rule(workout())).await;
    // The record of `record` is Class B: its title must not leave the hub.
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Morning run")],
        "2026-11-16T12:00:00Z",
    )
    .await;
    let sending = sending(&pool);
    assert_eq!(
        send_waiting(&sending, at("2026-11-16T12:00:30Z"))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        send_waiting(&sending, at("2026-11-16T12:01:30Z"))
            .await
            .unwrap(),
        0
    );

    let rows = outbox(&pool).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].0, "reminder");
    assert_eq!(rows[0].1["title"], "Anima");
    assert_eq!(rows[0].1["body"], "You have a reminder.");
    let stored = rows[0].1.to_string();
    assert!(
        !stored.contains("Morning run") && !stored.contains("Train"),
        "{stored}"
    );
    let events: Vec<String> = sqlx::query_scalar("select message from events.event_log")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(events, ["A reminder was sent."], "no name for Class B");
    let state: String = sqlx::query_scalar("select state from reminders.firing")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "sent");
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_008_a_firing_that_no_channel_takes_is_suppressed_with_its_reason(pool: PgPool) {
    add(&pool, rule(workout())).await;
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Run")],
        "2026-11-16T12:00:00Z",
    )
    .await;
    let sent = send_waiting(&sending(&pool), at("2026-11-16T12:00:30Z"))
        .await
        .unwrap();
    assert_eq!(sent, 0);
    let firing: (String, Option<String>, bool) =
        sqlx::query_as("select state, reason, sent_at is null from reminders.firing")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        firing,
        ("suppressed".to_owned(), Some(NO_CHANNEL.to_owned()), true)
    );
    assert_eq!(outbox(&pool).await, []);
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_005_a_rule_that_changed_before_the_send_queues_nothing(pool: PgPool) {
    default_channel(&pool).await;
    add(&pool, rule(workout())).await;
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Run")],
        "2026-11-16T12:00:00Z",
    )
    .await;
    sqlx::query("update reminders.rule set revision = revision + 1")
        .execute(&pool)
        .await
        .unwrap();
    let sent = send_waiting(&sending(&pool), at("2026-11-16T12:00:30Z"))
        .await
        .unwrap();
    assert_eq!(sent, 0);
    assert_eq!(outbox(&pool).await, []);
    let reason: Option<String> = sqlx::query_scalar("select reason from reminders.firing")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(reason.as_deref(), Some(RULE_CHANGED));
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_008_an_open_application_gets_the_firing_and_no_channel_is_queued(pool: PgPool) {
    default_channel(&pool).await;
    add(&pool, rule(workout())).await;
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Morning run")],
        "2026-11-16T12:00:00Z",
    )
    .await;
    let sending = sending(&pool);
    assert_eq!(sending.live.open(), 0);
    let mut application = sending.live.subscribe();
    assert_eq!(sending.live.open(), 1);
    let sent = send_waiting(&sending, at("2026-11-16T12:00:30Z"))
        .await
        .unwrap();
    assert_eq!(sent, 1);

    assert_eq!(outbox(&pool).await, [], "the application shows it");
    let shown = application.try_recv().unwrap();
    assert_eq!(shown.state, FiringState::Sent);
    assert_eq!(shown.sent_at, Some(at("2026-11-16T12:00:30Z")));
    // The record is Class B: its title and its subject stay in the hub (REM-009).
    assert_eq!(
        (shown.text.as_str(), shown.subject.as_str()),
        ("You have a reminder.", "")
    );
    let events: Vec<String> = sqlx::query_scalar("select message from events.event_log")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(events, ["A reminder was sent."]);

    // The application closed: the next firing goes to the channel.
    drop(application);
    assert_eq!(sending.live.open(), 0);
    noticed(
        &pool,
        &[record("strava", "workout", "2", "Evening run")],
        "2026-11-17T12:00:00Z",
    )
    .await;
    let sent = send_waiting(&sending, at("2026-11-17T12:00:30Z"))
        .await
        .unwrap();
    assert_eq!(sent, 1);
    assert_eq!(outbox(&pool).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_008_an_open_application_takes_a_firing_that_no_channel_takes(pool: PgPool) {
    add(&pool, rule(workout())).await;
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Run")],
        "2026-11-16T12:00:00Z",
    )
    .await;
    let sending = sending(&pool);
    let mut application = sending.live.subscribe();
    let sent = send_waiting(&sending, at("2026-11-16T12:00:30Z"))
        .await
        .unwrap();
    assert_eq!(sent, 1);
    assert!(application.try_recv().is_ok());
    let firing: (String, Option<String>) =
        sqlx::query_as("select state, reason from reminders.firing")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(firing, ("sent".to_owned(), None));
}

#[test]
fn rem_009_a_firing_of_class_c_goes_to_a_device_as_it_is_stored() {
    let stored = Firing {
        id: crate::ids::FiringId::from(Uuid::from_u128(7)),
        rule: ReminderRuleId::from(Uuid::from_u128(1)),
        revision: 1,
        subject: "check:pve-01".to_owned(),
        text: "The backup of pve-01 is late".to_owned(),
        class: Class::C,
        state: FiringState::Sent,
        reason: None,
        due_at: at("2026-11-16T12:00:00Z"),
        sent_at: Some(at("2026-11-16T12:00:30Z")),
        snoozed_until: None,
        decided_at: None,
    };
    assert_eq!(for_a_device(stored.clone()), stored);
    let private = for_a_device(Firing {
        class: Class::A,
        ..stored.clone()
    });
    assert_eq!(
        (private.text.as_str(), private.subject.as_str()),
        ("You have a reminder.", "")
    );
    assert_eq!(private.id, stored.id);
}
