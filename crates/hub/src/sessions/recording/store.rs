//! The only SQL for `sessions.recordings` (K2).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};

use super::{
    Error,
    format::Format,
    model::{self, RecordingState, StopReason, Stored},
};
use crate::{
    credentials::Sealed,
    ids::{DeviceId, RecordingId, SessionId},
    sessions::Protocol,
};

/// A recording's row as it is inserted, before its file has a part (K4 §11).
pub(super) struct NewRecording<'a> {
    pub(super) id: RecordingId,
    pub(super) session: SessionId,
    pub(super) device: DeviceId,
    pub(super) device_name: &'a str,
    pub(super) protocol: Protocol,
    pub(super) format: Format,
    pub(super) key_sealed: &'a Sealed,
    pub(super) started_at: DateTime<Utc>,
    pub(super) expires_at: DateTime<Utc>,
}

pub(super) async fn insert<'e>(
    db: impl PgExecutor<'e>,
    new: &NewRecording<'_>,
) -> Result<(), Error> {
    sqlx::query(
        "insert into sessions.recordings (id, session_id, device_id, device_name, protocol, \
                format, key_sealed, started_at, expires_at) \
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(new.id)
    .bind(new.session)
    .bind(new.device)
    .bind(new.device_name)
    .bind(new.protocol.as_str())
    .bind(model::format_text(new.format))
    .bind(new.key_sealed.to_bytes())
    .bind(new.started_at)
    .bind(new.expires_at)
    .execute(db)
    .await?;
    Ok(())
}

/// How a recording ended: at its session's end, or stopped early with a reason.
pub(super) struct Ending {
    pub(super) at: DateTime<Utc>,
    /// Plaintext bytes written in parts.
    pub(super) size_bytes: u64,
    /// The last part is in the file.
    pub(super) complete: bool,
    pub(super) stopped: Option<StopReason>,
}

/// Ends a recording once, at `ending.at` or its start if that is later. Returns whether this
/// call ended it.
pub(super) async fn end<'e>(
    db: impl PgExecutor<'e>,
    id: RecordingId,
    ending: &Ending,
) -> Result<bool, Error> {
    let size = i64::try_from(ending.size_bytes).unwrap_or(i64::MAX);
    let ended = sqlx::query(
        "update sessions.recordings \
         set ended_at = greatest($2, started_at), size_bytes = $3, complete = $4, \
             stopped_reason = $5 \
         where id = $1 and ended_at is null",
    )
    .bind(id)
    .bind(ending.at)
    .bind(size)
    .bind(ending.complete)
    .bind(ending.stopped.map(StopReason::as_str))
    .execute(db)
    .await?;
    Ok(ended.rows_affected() == 1)
}

/// The recording state of each of `sessions` (G-5): `Off` with no row, `Stopped` with a stop
/// reason, otherwise `On`. Every asked session has an entry.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Session.recording (WP-48 T5a) is the first caller"
    )
)]
pub(in crate::sessions) async fn states<'e>(
    db: impl PgExecutor<'e>,
    sessions: &[SessionId],
) -> Result<HashMap<SessionId, RecordingState>, Error> {
    let rows: Vec<(SessionId, bool)> = sqlx::query_as(
        "select session_id, stopped_reason is not null from sessions.recordings \
         where session_id = any($1)",
    )
    .bind(sessions)
    .fetch_all(db)
    .await?;
    let mut states: HashMap<_, _> = sessions
        .iter()
        .map(|session| (*session, RecordingState::Off))
        .collect();
    for (session, stopped) in rows {
        let state = if stopped {
            RecordingState::Stopped
        } else {
            RecordingState::On
        };
        states.insert(session, state);
    }
    Ok(states)
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "playback (WP-48 T6) reads stored recordings")
)]
#[derive(FromRow)]
struct RecordingRow {
    id: RecordingId,
    session_id: SessionId,
    device_id: Option<DeviceId>,
    device_name: String,
    protocol: String,
    format: String,
    key_sealed: Vec<u8>,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    size_bytes: i64,
    complete: bool,
    stopped_reason: Option<String>,
    expires_at: DateTime<Utc>,
}

impl TryFrom<RecordingRow> for Stored {
    type Error = Error;

    fn try_from(row: RecordingRow) -> Result<Self, Error> {
        let stopped = match row.stopped_reason.as_deref() {
            None => None,
            Some(text) => Some(StopReason::from_stored(text).ok_or(Error::StoredValue)?),
        };
        Ok(Self {
            id: row.id,
            session: row.session_id,
            device: row.device_id,
            device_name: row.device_name,
            protocol: row.protocol.parse().map_err(|_| Error::StoredValue)?,
            format: model::format_from_stored(&row.format).ok_or(Error::StoredValue)?,
            key_sealed: Sealed::from_bytes(&row.key_sealed).map_err(|_| Error::StoredValue)?,
            started_at: row.started_at,
            ended_at: row.ended_at,
            size_bytes: u64::try_from(row.size_bytes).map_err(|_| Error::StoredValue)?,
            complete: row.complete,
            stopped,
            expires_at: row.expires_at,
        })
    }
}

/// The recording with this id and its sealed key, for playback (K4 §12).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "playback (WP-48 T6) is the first caller")
)]
pub(in crate::sessions) async fn get<'e>(
    db: impl PgExecutor<'e>,
    id: RecordingId,
) -> Result<Option<Stored>, Error> {
    let row: Option<RecordingRow> = sqlx::query_as(
        "select id, session_id, device_id, device_name, protocol, format, key_sealed, \
                started_at, ended_at, size_bytes, complete, stopped_reason, expires_at \
         from sessions.recordings where id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(Stored::try_from).transpose()
}
