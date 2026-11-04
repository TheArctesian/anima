//! What a session leaves in the database: the end row, its `session.end` audit entries and
//! the events.

use std::time::Duration;

use serde_json::Value;
use sqlx::PgPool;

use crate::ids::SessionId;

/// What a session left behind once its row ended.
#[derive(Debug)]
pub(super) struct Ended {
    pub(super) reason: String,
    /// `(operator name, client address, change)` of each `session.end` entry.
    pub(super) audits: Vec<(String, String, Value)>,
    /// `(severity, source, device name, message)` of each event.
    pub(super) events: Vec<(String, String, Option<String>, String)>,
}

/// Waits up to five seconds for the session's row to end, then reads what the end wrote.
pub(super) async fn ended(pool: &PgPool, session: SessionId) -> Ended {
    for _ in 0..250 {
        let reason: Option<String> =
            sqlx::query_scalar("select end_reason from sessions.sessions where id = $1")
                .bind(session)
                .fetch_one(pool)
                .await
                .unwrap();
        if let Some(reason) = reason {
            // The audit entry and event commit with the row.
            return Ended {
                reason,
                audits: end_audits(pool, session).await,
                events: events(pool).await,
            };
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the session did not end");
}

async fn end_audits(pool: &PgPool, session: SessionId) -> Vec<(String, String, Value)> {
    sqlx::query_as(
        "select operator_name, host(client_addr), change from events.audit_log \
         where action = 'end' and target_type = 'session' and target_id = $1",
    )
    .bind(session.to_string())
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn events(pool: &PgPool) -> Vec<(String, String, Option<String>, String)> {
    sqlx::query_as("select severity, source, device_name, message from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// Every text the database holds about sessions, audit and events, as one string.
pub(super) async fn stored_text(pool: &PgPool) -> String {
    let rows: Vec<String> = sqlx::query_scalar(
        "select row_to_json(s)::text from sessions.sessions s \
         union all select row_to_json(a)::text from events.audit_log a \
         union all select row_to_json(e)::text from events.event_log e",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    rows.join("\n")
}
