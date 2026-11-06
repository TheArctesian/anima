//! All SQL on schema `sessions`: I/O only. Ids and time come in as parameters.

mod server_certificates;

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::{
    Error, Missing,
    model::{EndReason, HostKey, Protocol, Session, SessionEnd, SessionFilter},
    page::{Cursor, Page, split_page},
    public_key::{BadKey, PublicKey},
};
use crate::{
    events::{ClientAddr, Operator},
    ids::{DeviceId, OperatorId, SessionId},
};

pub(super) use server_certificates::server_certificates;
#[cfg_attr(
    not(test),
    expect(
        unused_imports,
        reason = "acceptServerCertificate (WP-22 T4) is the first caller outside the tests"
    )
)]
pub(super) use server_certificates::{AcceptedCertificate, put_server_certificate};

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

/// Ends a live session at `end.at`, or at its start if that is later (a clock that stepped
/// back must not leave the row live, MV-003). Returns whether this call ended it: a session
/// ends once, and the first end wins (D-9).
pub(super) async fn end<'e>(
    db: impl PgExecutor<'e>,
    id: SessionId,
    end: SessionEnd,
) -> Result<bool, Error> {
    let ended = sqlx::query(
        "update sessions.sessions set ended_at = greatest($2, started_at), end_reason = $3 \
         where id = $1 and ended_at is null",
    )
    .bind(id)
    .bind(end.at)
    .bind(end.reason.as_str())
    .execute(db)
    .await?;
    Ok(ended.rows_affected() == 1)
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
            operator: Operator { id, name },
            client: ClientAddr(client),
        })
        .collect();
    Ok(stale)
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

/// Waits for, then holds until the transaction ends, this domain's advisory lock on the
/// device's host key, so two accepts for one device run one after the other and each audit
/// entry names the key it replaced.
pub(super) async fn lock_host_key(conn: &mut PgConnection, device: DeviceId) -> Result<(), Error> {
    sqlx::query(
        "select pg_advisory_xact_lock(hashtextextended('sessions.host_keys:' || $1::text, 0))",
    )
    .bind(device)
    .execute(conn)
    .await?;
    Ok(())
}

/// A host key the operator accepts for a device.
pub(super) struct AcceptedKey<'a> {
    pub(super) device: DeviceId,
    pub(super) key: &'a HostKey,
    pub(super) by: OperatorId,
    pub(super) at: DateTime<Utc>,
}

/// Stores the accepted key, replacing the device's previous one. The algorithm and the
/// fingerprint are read from the key, so the row's checks always hold.
pub(super) async fn put_host_key(
    conn: &mut PgConnection,
    accepted: &AcceptedKey<'_>,
) -> Result<(), Error> {
    let public_key = &accepted.key.public_key;
    let stored = sqlx::query(
        "insert into sessions.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1, $2, $3, $4, $5, $6, $7) \
         on conflict (device_id) do update set algorithm = excluded.algorithm, \
             fingerprint_sha256 = excluded.fingerprint_sha256, public_key = excluded.public_key, \
             first_seen_at = excluded.first_seen_at, accepted_at = excluded.accepted_at, \
             accepted_by = excluded.accepted_by",
    )
    .bind(accepted.device)
    .bind(public_key.algorithm())
    .bind(public_key.fingerprint().as_str())
    .bind(public_key.to_base64())
    .bind(accepted.key.first_seen_at)
    .bind(accepted.at)
    .bind(accepted.by)
    .execute(conn)
    .await;
    match stored {
        Ok(_) => Ok(()),
        Err(error) if is_violation_of(&error, "host_keys_device_id_fkey") => {
            Err(Error::NotFound(Missing::Device(accepted.device)))
        }
        Err(error) => Err(error.into()),
    }
}

fn is_violation_of(error: &sqlx::Error, constraint: &str) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.constraint())
        == Some(constraint)
}
