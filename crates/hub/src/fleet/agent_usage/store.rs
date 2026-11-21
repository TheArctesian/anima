//! The only SQL of the schema `agent_usage` (K2 1020-1039). Every text from a device reaches a
//! statement as a bound value, never as SQL.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use daemon_core::agent_usage::{SessionDay, Source, UsageState};
use sqlx::{PgConnection, PgPool};

use super::{
    Error,
    model::{Ask, DeviceState, Dimension, DimensionSet, Grain, GroupKey, Sums, TokenSums},
};
use crate::ids::DeviceId;

mod sessions;
pub(super) use sessions::sessions;

/// Keeps `state` as the usage state of `device`; the same state again changes nothing.
pub(super) async fn write_state(
    conn: &mut PgConnection,
    device: DeviceId,
    state: UsageState,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO agent_usage.states (device_id, state) VALUES ($1, $2) \
         ON CONFLICT (device_id) DO UPDATE SET state = excluded.state \
         WHERE states.state <> excluded.state",
    )
    .bind(device)
    .bind(state.as_str())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Stores `days` of `device`: upserts each session with the start, end and repository of its
/// last entry in `days`, replaces the prompts, tool calls and model rows of each session day,
/// then sets the model of each session touched. Five statements whatever the number of entries.
pub(super) async fn write_session_days(
    conn: &mut PgConnection,
    device: DeviceId,
    days: &[SessionDay],
) -> Result<(), Error> {
    if days.is_empty() {
        return Ok(());
    }
    let ids = upsert_sessions(conn, device, days).await?;
    let session_of = |day: &SessionDay| ids.get(&(day.source, day.id.clone())).copied();
    let rows: Vec<(i64, &SessionDay)> = days
        .iter()
        .filter_map(|day| Some((session_of(day)?, day)))
        .collect();
    replace_days(conn, &rows).await?;
    let sessions: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
    sqlx::query(
        "UPDATE agent_usage.sessions s SET model = ( \
           SELECT m.model FROM agent_usage.day_models m WHERE m.session_id = s.id \
           GROUP BY m.model ORDER BY sum(m.output_tokens) DESC, m.model COLLATE \"C\" LIMIT 1) \
         WHERE s.id = ANY($1)",
    )
    .bind(&sessions)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Upserts one session for each source and ID of `days`, with the values of its last entry;
/// the row ID of each.
async fn upsert_sessions(
    conn: &mut PgConnection,
    device: DeviceId,
    days: &[SessionDay],
) -> Result<HashMap<(Source, String), i64>, Error> {
    let mut last: HashMap<(Source, &str), &SessionDay> = HashMap::new();
    for day in days {
        last.insert((day.source, day.id.as_str()), day);
    }
    let sessions: Vec<&SessionDay> = last.into_values().collect();
    let sources: Vec<&str> = sessions.iter().map(|day| day.source.as_str()).collect();
    let external: Vec<&str> = sessions.iter().map(|day| day.id.as_str()).collect();
    let starts: Vec<DateTime<Utc>> = sessions.iter().map(|day| day.started_at).collect();
    let ends: Vec<DateTime<Utc>> = sessions.iter().map(|day| day.ended_at).collect();
    let repositories: Vec<Option<&str>> = sessions
        .iter()
        .map(|day| day.repository.as_deref())
        .collect();
    let stored: Vec<(i64, String, String)> = sqlx::query_as(
        "INSERT INTO agent_usage.sessions \
           (device_id, source, external_id, started_at, ended_at, repository) \
         SELECT $1, source, external_id, started_at, ended_at, repository \
         FROM UNNEST($2::text[], $3::text[], $4::timestamptz[], $5::timestamptz[], $6::text[]) \
           AS s (source, external_id, started_at, ended_at, repository) \
         ON CONFLICT (device_id, source, external_id) DO UPDATE SET \
           started_at = excluded.started_at, ended_at = excluded.ended_at, \
           repository = excluded.repository \
         RETURNING id, source, external_id",
    )
    .bind(device)
    .bind(&sources)
    .bind(&external)
    .bind(&starts)
    .bind(&ends)
    .bind(&repositories)
    .fetch_all(&mut *conn)
    .await?;
    stored
        .into_iter()
        .map(|(id, source, external_id)| {
            let source = Source::parse(&source).ok_or(Error::Unreadable)?;
            Ok(((source, external_id), id))
        })
        .collect()
}

/// Replaces the stored day and model rows of each session day of `rows`.
async fn replace_days(conn: &mut PgConnection, rows: &[(i64, &SessionDay)]) -> Result<(), Error> {
    let sessions: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
    let days: Vec<NaiveDate> = rows.iter().map(|(_, day)| day.day).collect();
    let prompts: Vec<i64> = rows.iter().map(|(_, day)| count(day.prompts)).collect();
    let tool_calls: Vec<i64> = rows.iter().map(|(_, day)| count(day.tool_calls)).collect();
    sqlx::query(
        "DELETE FROM agent_usage.session_days d \
         USING UNNEST($1::bigint[], $2::date[]) AS k (session_id, day) \
         WHERE d.session_id = k.session_id AND d.day = k.day",
    )
    .bind(&sessions)
    .bind(&days)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO agent_usage.session_days (session_id, day, prompts, tool_calls) \
         SELECT * FROM UNNEST($1::bigint[], $2::date[], $3::bigint[], $4::bigint[])",
    )
    .bind(&sessions)
    .bind(&days)
    .bind(&prompts)
    .bind(&tool_calls)
    .execute(&mut *conn)
    .await?;
    insert_models(conn, rows).await
}

async fn insert_models(conn: &mut PgConnection, rows: &[(i64, &SessionDay)]) -> Result<(), Error> {
    let models: Vec<(i64, NaiveDate, &daemon_core::agent_usage::ModelTokens)> = rows
        .iter()
        .flat_map(|(id, day)| day.models.iter().map(move |one| (*id, day.day, one)))
        .collect();
    let column = |pick: fn(&daemon_core::agent_usage::Tokens) -> u64| -> Vec<i64> {
        models
            .iter()
            .map(|(_, _, one)| count(pick(&one.tokens)))
            .collect()
    };
    sqlx::query(
        "INSERT INTO agent_usage.day_models (session_id, day, model, input_tokens, \
           output_tokens, cache_read_tokens, cache_write_tokens) \
         SELECT * FROM UNNEST($1::bigint[], $2::date[], $3::text[], $4::bigint[], $5::bigint[], \
           $6::bigint[], $7::bigint[])",
    )
    .bind(models.iter().map(|(id, _, _)| *id).collect::<Vec<i64>>())
    .bind(
        models
            .iter()
            .map(|(_, day, _)| *day)
            .collect::<Vec<NaiveDate>>(),
    )
    .bind(
        models
            .iter()
            .map(|(_, _, one)| one.model.as_str())
            .collect::<Vec<&str>>(),
    )
    .bind(column(|tokens| tokens.input))
    .bind(column(|tokens| tokens.output))
    .bind(column(|tokens| tokens.cache_read))
    .bind(column(|tokens| tokens.cache_write))
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// A K8 count as a `bigint`. K8 keeps each count at most 2^53 − 1, so it always fits; a larger
/// one is held at `i64::MAX`, which the table's check refuses.
fn count(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[derive(sqlx::FromRow)]
struct GrainRow {
    device_id: Option<DeviceId>,
    source: Option<String>,
    repository: Option<String>,
    day: Option<NaiveDate>,
    model: Option<String>,
    sessions: i64,
    prompts: f64,
    tool_calls: f64,
    input_tokens: f64,
    output_tokens: f64,
    cache_read_tokens: f64,
    cache_write_tokens: f64,
}

/// The sums `ask` admits, grouped by the dimensions of `by` and always by model: the tokens of
/// each model, and the sessions, prompts and tool calls of each session under the session's
/// model. One statement; the dimensions only switch fixed `CASE` parts on and off.
pub(super) async fn grains(
    db: &PgPool,
    ask: &Ask,
    by: DimensionSet,
    limit: usize,
) -> Result<Vec<Grain>, Error> {
    let rows: Vec<GrainRow> = sqlx::query_as(
        "WITH days AS ( \
           SELECT s.id, d.day, d.prompts, d.tool_calls, s.model, \
             CASE WHEN $6::bool THEN s.device_id END AS device_id, \
             CASE WHEN $7::bool THEN s.source END AS source, \
             CASE WHEN $8::bool THEN s.repository END AS repository, \
             CASE WHEN $9::bool THEN d.day END AS group_day \
           FROM agent_usage.session_days d \
           JOIN agent_usage.sessions s ON s.id = d.session_id \
           WHERE d.day BETWEEN $1 AND $2 \
             AND ($3::uuid IS NULL OR s.device_id = $3) \
             AND ($4::text IS NULL OR s.source = $4) \
             AND ($5::text IS NULL OR s.repository = $5)), \
         grains AS ( \
           SELECT device_id, source, repository, group_day, model, \
             count(DISTINCT id) AS sessions, sum(prompts) AS prompts, \
             sum(tool_calls) AS tool_calls, 0::numeric AS input, 0::numeric AS output, \
             0::numeric AS cache_read, 0::numeric AS cache_write \
           FROM days GROUP BY 1, 2, 3, 4, 5 \
           UNION ALL \
           SELECT k.device_id, k.source, k.repository, k.group_day, m.model, \
             0, 0, 0, sum(m.input_tokens), sum(m.output_tokens), sum(m.cache_read_tokens), \
             sum(m.cache_write_tokens) \
           FROM days k JOIN agent_usage.day_models m \
             ON m.session_id = k.id AND m.day = k.day \
           GROUP BY 1, 2, 3, 4, 5) \
         SELECT device_id, source, repository, group_day AS day, model, \
           sum(sessions)::bigint AS sessions, sum(prompts)::float8 AS prompts, \
           sum(tool_calls)::float8 AS tool_calls, sum(input)::float8 AS input_tokens, \
           sum(output)::float8 AS output_tokens, sum(cache_read)::float8 AS cache_read_tokens, \
           sum(cache_write)::float8 AS cache_write_tokens \
         FROM grains GROUP BY 1, 2, 3, 4, 5 \
         LIMIT $10",
    )
    .bind(ask.first_day)
    .bind(ask.last_day)
    .bind(ask.device)
    .bind(ask.source.map(Source::as_str))
    .bind(ask.repository.as_deref())
    .bind(by.has(Dimension::Device))
    .bind(by.has(Dimension::Source))
    .bind(by.has(Dimension::Repository))
    .bind(by.has(Dimension::Day))
    .bind(i64::try_from(limit.saturating_add(1)).unwrap_or(i64::MAX))
    .fetch_all(db)
    .await?;
    if rows.len() > limit {
        return Err(Error::TooManyGroups);
    }
    rows.into_iter().map(Grain::try_from).collect()
}

impl TryFrom<GrainRow> for Grain {
    type Error = Error;

    fn try_from(row: GrainRow) -> Result<Self, Error> {
        let source = match row.source {
            Some(text) => Some(Source::parse(&text).ok_or(Error::Unreadable)?),
            None => None,
        };
        Ok(Self {
            key: GroupKey {
                device: row.device_id,
                source,
                model: row.model,
                repository: row.repository,
                day: row.day,
            },
            sums: Sums {
                sessions: row.sessions,
                prompts: row.prompts,
                tool_calls: row.tool_calls,
                tokens: TokenSums {
                    input: row.input_tokens,
                    output: row.output_tokens,
                    cache_read: row.cache_read_tokens,
                    cache_write: row.cache_write_tokens,
                },
            },
        })
    }
}

/// The usage state of `device`, or of every device when `None`.
pub(super) async fn states(
    db: &PgPool,
    device: Option<DeviceId>,
) -> Result<Vec<DeviceState>, Error> {
    let rows: Vec<(DeviceId, String)> = sqlx::query_as(
        "SELECT device_id, state FROM agent_usage.states \
         WHERE $1::uuid IS NULL OR device_id = $1",
    )
    .bind(device)
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(device, state)| {
            let state = UsageState::parse(&state).ok_or(Error::Unreadable)?;
            Ok(DeviceState { device, state })
        })
        .collect()
}
