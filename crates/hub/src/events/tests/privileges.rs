//! The hub role may append to the logs but never change or remove an entry (LOG-006, K2).

use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{day, device_event, new_audit};
use crate::events::{Bus, audit, record};

/// Runs `sql` and returns its SQLSTATE, or `None` if it succeeded.
async fn refusal(conn: &mut PgConnection, sql: &str) -> Option<String> {
    let error = sqlx::query(sql).execute(conn).await.err()?;
    let code = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    Some(code.map_or_else(|| error.to_string(), Into::into))
}

#[sqlx::test(migrations = "./migrations")]
async fn log_006_events_writer_appends_but_cannot_change_or_remove_entries(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let role = format!("anima_test_{}", Uuid::now_v7().simple());
    let create = format!("create role {role} nologin in role anima_events_writer");
    sqlx::query(&create).execute(&mut *conn).await.unwrap();
    sqlx::query(&format!("set role {role}"))
        .execute(&mut *conn)
        .await
        .unwrap();

    let at = day(2030, 5, 5);
    record(&mut conn, &Bus::new(1), at, device_event("as hub"))
        .await
        .unwrap();
    audit(&mut conn, at, new_audit(json!({ "name": "x" })))
        .await
        .unwrap();

    let attempts = [
        "update events.audit_log set operator_name = 'someone else'",
        "delete from events.audit_log",
        "truncate events.audit_log",
        "update events.audit_log_20300505 set operator_name = 'someone else'",
        "delete from events.audit_log_20300505",
        "drop table events.audit_log_20300505",
    ];
    let mut refusals = Vec::new();
    for sql in attempts {
        refusals.push((sql, refusal(&mut conn, sql).await));
    }

    sqlx::query("reset role").execute(&mut *conn).await.unwrap();
    sqlx::query(&format!("drop role {role}"))
        .execute(&mut *conn)
        .await
        .unwrap();
    for (sql, code) in refusals {
        assert_eq!(code.as_deref(), Some("42501"), "{sql}");
    }
}
