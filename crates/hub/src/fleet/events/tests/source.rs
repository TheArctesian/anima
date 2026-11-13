//! The `vpn` event source: stored, read back by a source filter and named `VPN` in GraphQL
//! (VPN-010, VPN-011, K1 `EventSource`).

use async_graphql::{InputType as _, Name, Value};
use chrono::{TimeZone as _, Utc};
use sqlx::PgPool;

use crate::fleet::events::{
    Bus, NewEvent, Severity, Source, model::EventFilter, record, store::list_events,
};

#[sqlx::test(migrations = "./migrations")]
async fn events_source_vpn_is_stored_and_read_back(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let at = Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap();
    let event = NewEvent {
        severity: Severity::Warning,
        source: Source::Vpn,
        device: None,
        message: "VPN node guest (100.64.0.8) is not in Anima".to_owned(),
        actor: crate::fleet::events::ActorKind::Hub,
    };
    record(&mut conn, &Bus::new(1), at, event).await.unwrap();
    let filter = EventFilter {
        severities: None,
        sources: Some(vec![Source::Vpn]),
        device: None,
        since: None,
    };

    let read = list_events(&pool, &filter, None, 10).await.unwrap();

    assert_eq!(read.len(), 1);
    assert_eq!(read[0].source, Source::Vpn);
    assert_eq!(read[0].source.to_value(), Value::Enum(Name::new("VPN")));
}

#[sqlx::test(migrations = "./migrations")]
async fn vpn_010_the_event_log_accepts_source_vpn_and_still_the_others(pool: PgPool) {
    sqlx::query("select events.ensure_day_partition('event_log', now())")
        .execute(&pool)
        .await
        .unwrap();
    let insert = |source: &str| {
        format!(
            "insert into events.event_log (id, at, severity, source, message) \
             values (gen_random_uuid(), now(), 'warning', '{source}', 'm')"
        )
    };

    for source in [
        "check",
        "session",
        "device",
        "access",
        "hub",
        "metrics",
        "notification",
        "vpn",
    ] {
        let inserted = sqlx::query(&insert(source)).execute(&pool).await;
        assert!(inserted.is_ok(), "{source}");
    }
    let refused = sqlx::query(&insert("mail"))
        .execute(&pool)
        .await
        .unwrap_err();
    let code = refused
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    assert_eq!(code.as_deref(), Some("23514"));
}
