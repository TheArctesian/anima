//! Tests of the notification store against a real database (K2 `notifications`, D-16, D-25).

mod observe;
mod outbox;
mod partitions;
mod privileges;
mod schema;
mod sealing;
mod store;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use super::{ChannelFields, ChannelKind, store as sql};
use crate::ids::{ChannelId, CheckId};

/// A fixture check (K6): the first one on the first fixture device.
const PING: &str = "0190a000-0000-7000-8003-000000000001";
const SSH_PORT: &str = "0190a000-0000-7000-8003-000000000002";

fn check(text: &str) -> CheckId {
    text.parse().unwrap()
}

fn fields(name: &str) -> ChannelFields {
    ChannelFields {
        name: name.to_owned(),
        kind: ChannelKind::Ntfy,
        on_by_default: false,
        settings: serde_json::json!({ "server": "https://ntfy.example.test" }),
        secrets: Some(vec![1, 2, 3]),
        title_template: None,
        body_template: None,
    }
}

/// Stores a channel named `name` and returns its id.
async fn channel(pool: &PgPool, name: &str) -> ChannelId {
    let id = ChannelId::from(Uuid::now_v7());
    let mut conn = pool.acquire().await.unwrap();
    sql::insert_channel(&mut conn, id, &fields(name), now(pool).await)
        .await
        .unwrap();
    id
}

async fn now(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A pool whose every connection runs as `anima_runtime`.
async fn runtime_pool(pool: &PgPool) -> PgPool {
    PgPoolOptions::new()
        .max_connections(4)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE anima_runtime").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

/// The SQLSTATE of the error `sql` fails with, if it fails.
async fn refusal(pool: &PgPool, sql: &str) -> Option<String> {
    let error = sqlx::query(sql).execute(pool).await.err()?;
    error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .map(|code| code.to_string())
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM notifications.{table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}
