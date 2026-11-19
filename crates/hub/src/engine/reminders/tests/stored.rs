//! `notice` and the check at send time against the database (REM-002, REM-004, REM-005).

use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ids::FiringId;

async fn noticed(pool: &PgPool, new: &[Occurrence], now: &str) {
    let mut tx = pool.begin().await.unwrap();
    notice(&mut tx, new, at(now)).await.unwrap();
    tx.commit().await.unwrap();
}

/// `(subject, state, reason)` of each firing, oldest first.
async fn firings(pool: &PgPool) -> Vec<(String, String, Option<String>)> {
    sqlx::query_as("select subject, state, reason from reminders.firing order by due_at, subject")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn first_waiting(pool: &PgPool, now: &str) -> FiringId {
    let mut conn = pool.acquire().await.unwrap();
    waiting(&mut conn, at(now), 10).await.unwrap()[0]
}

async fn claimed(pool: &PgPool, firing: FiringId, now: &str) -> Option<Sendable> {
    let mut tx = pool.begin().await.unwrap();
    let sendable = claim_send(&mut tx, firing, at(now)).await.unwrap();
    tx.commit().await.unwrap();
    sendable
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_002_new_records_fire_the_rules_one_time_and_the_cooldown_is_stored(pool: PgPool) {
    add(&pool, rule(workout())).await;
    let first = [record("strava", "workout", "1", "Morning run")];
    noticed(&pool, &first, "2026-11-16T12:00:00Z").await;
    noticed(&pool, &first, "2026-11-16T12:00:05Z").await;
    assert_eq!(
        firings(&pool).await,
        [("workout:1".to_owned(), "pending".to_owned(), None)],
        "the same record again gives no second firing"
    );
    let second = [record("strava", "workout", "2", "Evening run")];
    noticed(&pool, &second, "2026-11-16T12:30:00Z").await;
    assert_eq!(
        firings(&pool).await[1],
        (
            "workout:2".to_owned(),
            "suppressed".to_owned(),
            Some("cooldown".to_owned())
        )
    );
    noticed(
        &pool,
        &[record("strava", "workout", "3", "Night run")],
        "2026-11-16T13:00:01Z",
    )
    .await;
    assert_eq!(
        firings(&pool).await[2].1,
        "pending",
        "a suppressed firing uses no cooldown"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_003_a_record_that_arrived_before_is_known_for_unless(pool: PgPool) {
    let reminder = Rule {
        unless: Some(Unless::RecordToday {
            source: "strava".to_owned(),
            kind: "workout".to_owned(),
        }),
        ..rule(Trigger::SourceEvent {
            source: "calendar".to_owned(),
            kind: "evening".to_owned(),
            contains: None,
        })
    };
    add(&pool, reminder).await;
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Run")],
        "2026-11-16T08:00:00Z",
    )
    .await;
    noticed(
        &pool,
        &[record("calendar", "evening", "1", "Evening")],
        "2026-11-16T18:00:00Z",
    )
    .await;
    assert_eq!(firings(&pool).await, [], "a workout of today is known");
    // The next day the workout of yesterday does not count.
    let next = Occurrence::Record {
        source: "calendar".to_owned(),
        kind: "evening".to_owned(),
        record: Reference::parse("evening:2").unwrap(),
        title: "Evening".to_owned(),
        class: Class::C,
        at: at("2026-11-17T18:00:00Z"),
    };
    noticed(&pool, &[next], "2026-11-17T18:00:00Z").await;
    assert_eq!(firings(&pool).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_005_a_firing_is_sent_only_when_its_rule_did_not_change(pool: PgPool) {
    add(&pool, rule(workout())).await;
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Morning run")],
        "2026-11-16T12:00:00Z",
    )
    .await;
    let firing = first_waiting(&pool, "2026-11-16T12:00:10Z").await;
    let sent = claimed(&pool, firing, "2026-11-16T12:00:10Z")
        .await
        .unwrap();
    assert_eq!(
        (sent.rule_name.as_str(), sent.text.as_str(), sent.class),
        ("Train", "Time to train: Morning run", Class::B)
    );
    assert_eq!(firings(&pool).await[0].1, "sent");
    assert_eq!(
        claimed(&pool, firing, "2026-11-16T12:00:20Z").await,
        None,
        "sent one time"
    );

    // The operator changes the rule after the next firing is due and before it is sent.
    noticed(
        &pool,
        &[record("strava", "workout", "2", "Late run")],
        "2026-11-16T14:00:00Z",
    )
    .await;
    sqlx::query("update reminders.rule set revision = revision + 1")
        .execute(&pool)
        .await
        .unwrap();
    let changed = first_waiting(&pool, "2026-11-16T14:00:10Z").await;
    assert_eq!(claimed(&pool, changed, "2026-11-16T14:00:10Z").await, None);
    assert_eq!(
        firings(&pool).await[1],
        (
            "workout:2".to_owned(),
            "suppressed".to_owned(),
            Some(RULE_CHANGED.to_owned())
        )
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_005_a_firing_of_a_rule_that_is_off_or_gone_is_not_sent(pool: PgPool) {
    add(&pool, rule(workout())).await;
    noticed(
        &pool,
        &[record("strava", "workout", "1", "Run")],
        "2026-11-16T12:00:00Z",
    )
    .await;
    let firing = first_waiting(&pool, "2026-11-16T12:00:10Z").await;
    sqlx::query("update reminders.rule set enabled = false")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(claimed(&pool, firing, "2026-11-16T12:00:10Z").await, None);
    assert_eq!(firings(&pool).await[0].2.as_deref(), Some(RULE_CHANGED));
    sqlx::query("delete from reminders.rule")
        .execute(&pool)
        .await
        .unwrap();
    let gone = FiringId::from(Uuid::now_v7());
    assert_eq!(claimed(&pool, gone, "2026-11-16T12:00:10Z").await, None);
    assert_eq!(firings(&pool).await, [], "the firings go with their rule");
}
