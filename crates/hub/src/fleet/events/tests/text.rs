//! The text filter of the event log: the message or the device name, with case ignored (LOG-011).

use chrono::{TimeZone as _, Utc};
use sqlx::PgPool;

use crate::fleet::events::{
    ActorKind, Bus, NewEvent, Severity, Source, model::EventFilter, record, store::list_events,
};

async fn recorded(pool: &PgPool, messages: &[&str]) {
    let mut conn = pool.acquire().await.unwrap();
    for (second, message) in (0..).zip(messages) {
        let at = Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, second).unwrap();
        let event = NewEvent {
            severity: Severity::Info,
            source: Source::Hub,
            device: None,
            message: (*message).to_owned(),
            actor: ActorKind::Hub,
        };
        record(&mut conn, &Bus::new(1), at, event).await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_011_the_text_filter_finds_the_message_with_case_ignored(pool: PgPool) {
    recorded(
        &pool,
        &["Backup complete: 41 MB", "check icmp failed", "100% done"],
    )
    .await;
    let search = |text: &str| EventFilter {
        text: Some(text.to_owned()),
        ..EventFilter::default()
    };

    let found = list_events(&pool, &search("BACKUP"), None, 10)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].message, "Backup complete: 41 MB");
    // `%` and `_` are plain characters, not patterns.
    let percent = list_events(&pool, &search("0%"), None, 10).await.unwrap();
    assert_eq!(percent.len(), 1);
    let pattern = list_events(&pool, &search("b_ckup"), None, 10)
        .await
        .unwrap();
    assert_eq!(pattern.len(), 0);
    let all = list_events(&pool, &EventFilter::default(), None, 10)
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
}
