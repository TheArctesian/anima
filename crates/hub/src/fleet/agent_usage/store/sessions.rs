//! The SQL of the single sessions (AIS-025): the sessions that have use on the days asked for,
//! newest use first, and the tokens of each by model. Every text from a device reaches a
//! statement as a bound value, never as SQL.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use daemon_core::agent_usage::Source;
use sqlx::PgPool;

use super::super::{
    Error,
    model::{Ask, SessionList, StoredSession, TokenSums},
    sessions::priced,
};
use crate::ids::DeviceId;

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: i64,
    device_id: DeviceId,
    source: String,
    model: Option<String>,
    repository: Option<String>,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    prompts: f64,
    tool_calls: f64,
    admitted: i64,
}

#[derive(sqlx::FromRow)]
struct ModelRow {
    session_id: i64,
    model: String,
    input_tokens: f64,
    output_tokens: f64,
    cache_read_tokens: f64,
    cache_write_tokens: f64,
}

/// At most `limit` sessions that `ask` admits, the newest last use first (the row ID breaks a
/// tie), with their sums over the days of `ask`, and how many it admits in all. Two statements.
pub(in super::super) async fn sessions(
    db: &PgPool,
    ask: &Ask,
    limit: usize,
) -> Result<SessionList, Error> {
    let rows = admitted(db, ask, limit).await?;
    let admitted = rows.first().map_or(0, |row| row.admitted);
    let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    let mut by_session = model_sums(db, ask, &ids).await?;
    let sessions = rows
        .into_iter()
        .map(|row| {
            let source = Source::parse(&row.source).ok_or(Error::Unreadable)?;
            Ok(priced(StoredSession {
                id: row.id,
                device: row.device_id,
                source,
                model: row.model,
                repository: row.repository,
                started_at: row.started_at,
                last_use_at: row.ended_at,
                prompts: row.prompts,
                tool_calls: row.tool_calls,
                models: by_session.remove(&row.id).unwrap_or_default(),
            }))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(SessionList { sessions, admitted })
}

/// The sessions with use on the days of `ask`, newest use first, each with its prompts and tool
/// calls on those days and the count of all that `ask` admits.
async fn admitted(db: &PgPool, ask: &Ask, limit: usize) -> Result<Vec<SessionRow>, Error> {
    let rows = sqlx::query_as(
        "SELECT s.id, s.device_id, s.source, s.model, s.repository, s.started_at, s.ended_at, \
           sum(d.prompts)::float8 AS prompts, sum(d.tool_calls)::float8 AS tool_calls, \
           count(*) OVER ()::bigint AS admitted \
         FROM agent_usage.session_days d \
         JOIN agent_usage.sessions s ON s.id = d.session_id \
         WHERE d.day BETWEEN $1 AND $2 \
           AND ($3::uuid IS NULL OR s.device_id = $3) \
           AND ($4::text IS NULL OR s.source = $4) \
           AND ($5::text IS NULL OR s.repository = $5) \
         GROUP BY s.id \
         ORDER BY s.ended_at DESC, s.id DESC \
         LIMIT $6",
    )
    .bind(ask.first_day)
    .bind(ask.last_day)
    .bind(ask.device)
    .bind(ask.source.map(Source::as_str))
    .bind(ask.repository.as_deref())
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// The tokens of each of the sessions `ids` by model, summed over the days of `ask`.
async fn model_sums(
    db: &PgPool,
    ask: &Ask,
    ids: &[i64],
) -> Result<HashMap<i64, Vec<(String, TokenSums)>>, Error> {
    let models: Vec<ModelRow> = sqlx::query_as(
        "SELECT session_id, model, sum(input_tokens)::float8 AS input_tokens, \
           sum(output_tokens)::float8 AS output_tokens, \
           sum(cache_read_tokens)::float8 AS cache_read_tokens, \
           sum(cache_write_tokens)::float8 AS cache_write_tokens \
         FROM agent_usage.day_models \
         WHERE session_id = ANY($1) AND day BETWEEN $2 AND $3 \
         GROUP BY session_id, model ORDER BY session_id, model COLLATE \"C\"",
    )
    .bind(ids)
    .bind(ask.first_day)
    .bind(ask.last_day)
    .fetch_all(db)
    .await?;
    let mut by_session: HashMap<i64, Vec<(String, TokenSums)>> = HashMap::new();
    for row in models {
        let sums = TokenSums {
            input: row.input_tokens,
            output: row.output_tokens,
            cache_read: row.cache_read_tokens,
            cache_write: row.cache_write_tokens,
        };
        by_session
            .entry(row.session_id)
            .or_default()
            .push((row.model, sums));
    }
    Ok(by_session)
}
