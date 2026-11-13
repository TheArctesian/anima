//! All SQL on schema `sessions`: I/O only. Ids and time come in as parameters.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::{
    Error,
    model::{EndReason, Protocol, Session, SessionEnd, SessionFilter},
    page::{Cursor, Page, split_page},
};
use crate::{
    events::{Actor, ClientAddr, Operator},
    ids::{DeviceId, OperatorId, SessionId},
};

const SESSION_COLUMNS: &str = "id, device_id, device_name, protocol, operator_id, operator_name, \
                               client_addr, started_at, ended_at, end_reason";

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: SessionId,
    device_id: Option<DeviceId>,
    device_name: String,
    protocol: String,
    operator_id: OperatorId,
    operator_name: String,
    client_addr: IpAddr,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    end_reason: Option<String>,
}

/// The row holds a half-written end: a time without a reason or a reason without a time.
#[derive(Debug, thiserror::Error)]
#[error("session row has ended_at and end_reason out of step")]
struct HalfEnded;

fn decode<E: std::error::Error + Send + Sync + 'static>(error: E) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(error))
}

impl TryFrom<SessionRow> for Session {
    type Error = sqlx::Error;

    fn try_from(row: SessionRow) -> Result<Self, sqlx::Error> {
        let end = match (row.ended_at, row.end_reason) {
            (None, None) => None,
            (Some(at), Some(reason)) => Some(SessionEnd {
                at,
                reason: reason.parse::<EndReason>().map_err(decode)?,
            }),
            _ => return Err(decode(HalfEnded)),
        };
        Ok(Self {
            id: row.id,
            device: row.device_id,
            device_name: row.device_name,
            protocol: row.protocol.parse::<Protocol>().map_err(decode)?,
            operator: row.operator_id,
            operator_name: row.operator_name,
            client: row.client_addr,
            started_at: row.started_at,
            end,
        })
    }
}

/// One page of sessions, newest first, after the cursor's row.
pub(super) async fn page<'e>(
    db: impl PgExecutor<'e>,
    filter: SessionFilter,
    size: usize,
    after: Option<Cursor>,
) -> Result<Page<Session>, Error> {
    let limit = i64::try_from(size).unwrap_or(i64::MAX).saturating_add(1);
    let rows: Vec<SessionRow> = sqlx::query_as(&format!(
        "select {SESSION_COLUMNS} from sessions.sessions \
         where ($1::uuid is null or device_id = $1) \
           and ($2::boolean is null or (ended_at is null) = $2) \
           and ($3::timestamptz is null or (started_at, id) < ($3, $4::uuid)) \
         order by started_at desc, id desc limit $5"
    ))
    .bind(filter.device)
    .bind(filter.live)
    .bind(after.map(|cursor| cursor.started_at))
    .bind(after.map(|cursor| cursor.id))
    .bind(limit)
    .fetch_all(db)
    .await?;
    let sessions = rows
        .into_iter()
        .map(Session::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(split_page(sessions, size))
}

/// What a new session row holds; it starts live.
pub(super) struct NewSession<'a> {
    pub(super) id: SessionId,
    pub(super) device: DeviceId,
    pub(super) device_name: &'a str,
    pub(super) protocol: Protocol,
    pub(super) operator: &'a Operator,
    pub(super) client: IpAddr,
    pub(super) started_at: DateTime<Utc>,
}

pub(super) async fn insert<'e>(
    db: impl PgExecutor<'e>,
    new: &NewSession<'_>,
) -> Result<Session, Error> {
    let row: SessionRow = sqlx::query_as(&format!(
        "insert into sessions.sessions (id, device_id, device_name, protocol, operator_id, \
                operator_name, client_addr, started_at) \
         values ($1, $2, $3, $4, $5, $6, $7, $8) returning {SESSION_COLUMNS}"
    ))
    .bind(new.id)
    .bind(new.device)
    .bind(new.device_name)
    .bind(new.protocol.as_str())
    .bind(new.operator.id)
    .bind(&new.operator.name)
    .bind(new.client)
    .bind(new.started_at)
    .fetch_one(db)
    .await?;
    Ok(Session::try_from(row)?)
}

/// Holds the device's start lock until the transaction ends, so starts on one device count
/// its live rows one at a time (RA-036). The key is the hub's advisory lock scheme, a 64-bit
/// hash of a namespace and the id (as in `trust` and `credentials`); the `sessions.start:`
/// namespace keeps it apart from every other lock. A hash collision only makes two starts
/// wait on each other, never lets one through.
pub(super) async fn lock_starts(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    sqlx::query("select pg_advisory_xact_lock(hashtextextended('sessions.start:' || $1::text, 0))")
        .bind(device)
        .execute(conn)
        .await?;
    Ok(())
}

/// How many of the device's sessions are live: rows with no end yet.
pub(super) async fn live_count<'e>(
    db: impl PgExecutor<'e>,
    device: DeviceId,
) -> Result<i64, Error> {
    let count = sqlx::query_scalar(
        "select count(*) from sessions.sessions where device_id = $1 and ended_at is null",
    )
    .bind(device)
    .fetch_one(db)
    .await?;
    Ok(count)
}

/// Ends a live session at `end.at`, or at its start if that is later (a clock that stepped
/// back must not leave the row live, MV-003). Returns the row as this call ended it, or
/// `None` when it did not: a session ends once, and the first end wins.
pub(super) async fn end<'e>(
    db: impl PgExecutor<'e>,
    id: SessionId,
    end: SessionEnd,
) -> Result<Option<Session>, Error> {
    let row: Option<SessionRow> = sqlx::query_as(&format!(
        "update sessions.sessions set ended_at = greatest($2, started_at), end_reason = $3 \
         where id = $1 and ended_at is null returning {SESSION_COLUMNS}"
    ))
    .bind(id)
    .bind(end.at)
    .bind(end.reason.as_str())
    .fetch_optional(db)
    .await?;
    Ok(row.map(Session::try_from).transpose()?)
}

/// The session with this id, live or ended.
pub(super) async fn get<'e>(
    db: impl PgExecutor<'e>,
    id: SessionId,
) -> Result<Option<Session>, Error> {
    let row: Option<SessionRow> = sqlx::query_as(&format!(
        "select {SESSION_COLUMNS} from sessions.sessions where id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(Session::try_from).transpose()?)
}

/// A live row `close_stale` ended: the session and who started it, from where.
pub(super) struct Stale {
    pub(super) session: SessionId,
    pub(super) operator: Operator,
    pub(super) client: ClientAddr,
}

/// Ends every live session with `HUB_SHUTDOWN` at `now`, or at its start if that is later,
/// so a row started ahead of a clock that stepped back still ends; returns each row it ended.
pub(super) async fn close_stale<'e>(
    db: impl PgExecutor<'e>,
    now: DateTime<Utc>,
) -> Result<Vec<Stale>, Error> {
    let rows: Vec<(SessionId, OperatorId, String, IpAddr)> = sqlx::query_as(
        "update sessions.sessions set ended_at = greatest($1, started_at), end_reason = $2 \
         where ended_at is null returning id, operator_id, operator_name, client_addr",
    )
    .bind(now)
    .bind(EndReason::HubShutdown.as_str())
    .fetch_all(db)
    .await?;
    let stale = rows
        .into_iter()
        .map(|(session, id, name, client)| Stale {
            session,
            // The hub ends a session a previous process left live (LOG-014 rule 1).
            operator: Operator {
                id,
                name,
                actor: Actor::hub(),
            },
            client: ClientAddr(client),
        })
        .collect();
    Ok(stale)
}
