//! All SQL on schema `sessions`: I/O only. Ids and time come in as parameters.

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use super::{
    Error,
    model::{EndReason, HostKey, Protocol, Session, SessionEnd, SessionFilter},
    page::{Cursor, Page, split_page},
    public_key::{BadKey, PublicKey},
};
use crate::{
    events::Operator,
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

/// The stored algorithm differs from the one inside the stored key.
#[derive(Debug, thiserror::Error)]
#[error("host key row names another algorithm than its key")]
struct AlgorithmMismatch;

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
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "WP-10 T2 `start` inserts the row")
)]
pub(super) struct NewSession<'a> {
    pub(super) id: SessionId,
    pub(super) device: DeviceId,
    pub(super) device_name: &'a str,
    pub(super) protocol: Protocol,
    pub(super) operator: &'a Operator,
    pub(super) client: IpAddr,
    pub(super) started_at: DateTime<Utc>,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "WP-10 T2 `start` inserts the row")
)]
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

/// Ends a live session. Returns whether this call ended it: a session ends once, and the
/// first end wins (D-9).
#[cfg_attr(not(test), expect(dead_code, reason = "WP-10 T2 and T5 end sessions"))]
pub(super) async fn end<'e>(
    db: impl PgExecutor<'e>,
    id: SessionId,
    end: SessionEnd,
) -> Result<bool, Error> {
    let ended = sqlx::query(
        "update sessions.sessions set ended_at = $2, end_reason = $3 \
         where id = $1 and ended_at is null",
    )
    .bind(id)
    .bind(end.at)
    .bind(end.reason.as_str())
    .execute(db)
    .await?;
    Ok(ended.rows_affected() == 1)
}

/// Ends every live session with `HUB_SHUTDOWN` at `now`; returns how many it ended.
pub(super) async fn close_stale<'e>(
    db: impl PgExecutor<'e>,
    now: DateTime<Utc>,
) -> Result<u64, Error> {
    let closed = sqlx::query(
        "update sessions.sessions set ended_at = $1, end_reason = $2 where ended_at is null",
    )
    .bind(now)
    .bind(EndReason::HubShutdown.as_str())
    .execute(db)
    .await?;
    Ok(closed.rows_affected())
}

#[derive(sqlx::FromRow)]
struct HostKeyRow {
    device_id: DeviceId,
    algorithm: String,
    public_key: String,
    first_seen_at: DateTime<Utc>,
}

impl TryFrom<HostKeyRow> for (DeviceId, HostKey) {
    type Error = sqlx::Error;

    fn try_from(row: HostKeyRow) -> Result<Self, sqlx::Error> {
        let public_key =
            PublicKey::from_base64(&row.public_key).map_err(|error: BadKey| decode(error))?;
        if public_key.algorithm() != row.algorithm {
            return Err(decode(AlgorithmMismatch));
        }
        let key = HostKey {
            public_key,
            first_seen_at: row.first_seen_at,
        };
        Ok((row.device_id, key))
    }
}

/// The accepted host key of each listed device that has one, in one query (API-005).
pub(super) async fn host_keys<'e>(
    db: impl PgExecutor<'e>,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, HostKey>, Error> {
    if devices.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<HostKeyRow> = sqlx::query_as(
        "select device_id, algorithm, public_key, first_seen_at from sessions.host_keys \
         where device_id = any($1)",
    )
    .bind(devices)
    .fetch_all(db)
    .await?;
    let keys = rows
        .into_iter()
        .map(<(DeviceId, HostKey)>::try_from)
        .collect::<Result<_, _>>()?;
    Ok(keys)
}
