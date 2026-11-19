//! What the engine gives with no extension: the events of the event log (K-E6 Section 1).

use sqlx::PgPool;

use super::*;
use crate::{
    engine::{locations::Seal, sealing_keys::SealingKeys},
    fleet::events::{self, ActorKind, Bus, NewEvent, Severity, Source},
};

fn reading() -> Reading {
    Reading {
        seal: Seal::new(SealingKeys::none()),
        extensions: Vec::new(),
    }
}

async fn event(pool: &PgPool, (source, severity): (Source, Severity), message: &str, when: &str) {
    let mut tx = pool.begin().await.unwrap();
    let new = NewEvent {
        severity,
        source,
        device: None,
        message: message.to_owned(),
        actor: ActorKind::Hub,
    };
    events::record(&mut tx, &Bus::new(4), at(when), new)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

async fn texts(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as("select text, class from reminders.firing order by due_at, subject")
        .fetch_all(pool)
        .await
        .unwrap()
}

fn on_events(source: &str, kind: &str, contains: Option<&str>) -> Rule {
    Rule {
        message: "Look: {title}".to_owned(),
        ..rule(Trigger::SourceEvent {
            source: source.to_owned(),
            kind: kind.to_owned(),
            contains: contains.map(str::to_owned),
        })
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_002_a_new_event_of_the_log_fires_a_rule_one_time(pool: PgPool) {
    add(&pool, on_events("check", "error", Some("DOWN"))).await;
    // An event from before the first sweep fires nothing.
    event(
        &pool,
        (Source::Check, Severity::Error),
        "web on pve-01 is down",
        "2026-11-19T11:00:00Z",
    )
    .await;
    sweep(&pool, &reading(), at("2026-11-19T12:00:00Z"))
        .await
        .unwrap();
    assert_eq!(texts(&pool).await, []);

    event(
        &pool,
        (Source::Check, Severity::Error),
        "db on pve-02 is down",
        "2026-11-19T12:00:10Z",
    )
    .await;
    event(
        &pool,
        (Source::Check, Severity::Info),
        "db on pve-02 is down",
        "2026-11-19T12:00:11Z",
    )
    .await;
    event(
        &pool,
        (Source::Check, Severity::Error),
        "db on pve-02 is slow",
        "2026-11-19T12:00:12Z",
    )
    .await;
    for now in ["2026-11-19T12:01:00Z", "2026-11-19T12:02:00Z"] {
        sweep(&pool, &reading(), at(now)).await.unwrap();
    }
    assert_eq!(
        texts(&pool).await,
        [("Look: db on pve-02 is down".to_owned(), "C".to_owned())]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn rem_002_the_event_of_a_firing_fires_no_rule(pool: PgPool) {
    add(&pool, on_events("hub", "info", None)).await;
    sweep(&pool, &reading(), at("2026-11-19T12:00:00Z"))
        .await
        .unwrap();
    event(
        &pool,
        (Source::Hub, Severity::Info),
        "A reminder was sent.",
        "2026-11-19T12:00:10Z",
    )
    .await;
    event(
        &pool,
        (Source::Hub, Severity::Info),
        "The reminder Train was sent.",
        "2026-11-19T12:00:11Z",
    )
    .await;
    sweep(&pool, &reading(), at("2026-11-19T12:01:00Z"))
        .await
        .unwrap();
    assert_eq!(texts(&pool).await, []);
    event(
        &pool,
        (Source::Hub, Severity::Info),
        "The hub started.",
        "2026-11-19T12:01:10Z",
    )
    .await;
    sweep(&pool, &reading(), at("2026-11-19T12:02:00Z"))
        .await
        .unwrap();
    assert_eq!(texts(&pool).await.len(), 1);
}
