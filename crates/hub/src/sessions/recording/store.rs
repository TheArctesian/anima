//! The only SQL for `sessions.recordings` (K2).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor};

use super::{
    Error,
    format::Format,
    model::{self, Catalogued, Position, RecordingState, StopReason, Stored},
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

/// The columns of a stored recording, with its sealed key.
const STORED_COLUMNS: &str = "id, session_id, device_id, device_name, protocol, format, \
                              key_sealed, started_at, ended_at, size_bytes, complete, \
                              stopped_reason, expires_at";

/// The recording with this id and its sealed key, whether or not the catalogue shows it.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "playback (WP-48 T6) is the first caller")
)]
pub(in crate::sessions) async fn get<'e>(
    db: impl PgExecutor<'e>,
    id: RecordingId,
) -> Result<Option<Stored>, Error> {
    let row: Option<RecordingRow> = sqlx::query_as(&format!(
        "select {STORED_COLUMNS} from sessions.recordings where id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(Stored::try_from).transpose()
}

/// The recording with this id and its sealed key when the catalogue shows it at `now`: what
/// playback reads again as it redeems a ticket, so a recording deleted, or ended and expired,
/// since the ticket was issued is not played (K4 §12, G-7, D-25).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "playback (WP-48 T6) reads the row again on redeem"
    )
)]
pub(in crate::sessions) async fn playable<'e>(
    db: impl PgExecutor<'e>,
    id: RecordingId,
    now: DateTime<Utc>,
) -> Result<Option<Stored>, Error> {
    let row: Option<RecordingRow> = sqlx::query_as(&format!(
        "select {STORED_COLUMNS} from sessions.recordings where {shown} and id = $2",
        shown = shown(),
    ))
    .bind(now)
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(Stored::try_from).transpose()
}

/// The columns of a catalogued recording, without the sealed key.
const CATALOGUED_COLUMNS: &str = "id, session_id, device_id, device_name, protocol, format, \
                                  started_at, ended_at, size_bytes, complete, expires_at";

#[derive(FromRow)]
struct CataloguedRow {
    id: RecordingId,
    session_id: SessionId,
    device_id: Option<DeviceId>,
    device_name: String,
    protocol: String,
    format: String,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    size_bytes: i64,
    complete: bool,
    expires_at: DateTime<Utc>,
}

impl TryFrom<CataloguedRow> for Catalogued {
    type Error = Error;

    fn try_from(row: CataloguedRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            session: row.session_id,
            device: row.device_id,
            device_name: row.device_name,
            protocol: row.protocol.parse().map_err(|_| Error::StoredValue)?,
            format: model::format_from_stored(&row.format).ok_or(Error::StoredValue)?,
            started_at: row.started_at,
            ended_at: row.ended_at,
            size_bytes: u64::try_from(row.size_bytes).map_err(|_| Error::StoredValue)?,
            complete: row.complete,
            expires_at: row.expires_at,
        })
    }
}

/// The SQL condition "this recording has ended", on a row of `sessions.recordings` not given
/// an alias: its own end is written, or its session has ended. A row whose end failed to
/// write, or that a hub crash left open, so ends with its session; at start the hub ends
/// every session the crash left open (`close_stale`). Live means not ended.
const ENDED: &str = "(ended_at is not null or exists (select 1 from sessions.sessions s \
                     where s.id = recordings.session_id and s.ended_at is not null))";

/// The SQL condition "the catalogue shows this recording at `$1`", the time of the request:
/// every one not yet expired, and every live one. A live recording stays while it is
/// written, even past its `expires_at`, as the daily retention job keeps its row and file
/// until it ends (CR-23 e); once it has ended and expired it is gone from the catalogue and
/// from playback, though its row waits for that job (D-25).
fn shown() -> String {
    format!("(not {ENDED} or expires_at > $1)")
}

/// Up to `limit` recordings shown at `now` after `after`, newest first, of `device` or of
/// every device (K1 `recordings`, API-006).
pub(super) async fn page<'e>(
    db: impl PgExecutor<'e>,
    (device, after): (Option<DeviceId>, Option<Position>),
    limit: i64,
    now: DateTime<Utc>,
) -> Result<Vec<Catalogued>, Error> {
    let rows: Vec<CataloguedRow> = sqlx::query_as(&format!(
        "select {CATALOGUED_COLUMNS} from sessions.recordings \
         where {shown} and ($2::uuid is null or device_id = $2) \
           and ($3::timestamptz is null or (started_at, id) < ($3, $4::uuid)) \
         order by started_at desc, id desc limit $5",
        shown = shown(),
    ))
    .bind(now)
    .bind(device)
    .bind(after.map(|position| position.started_at))
    .bind(after.map(|position| position.id))
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Catalogued::try_from).collect()
}

/// The recording with this id, without its key, when it is shown at `now`. The row is
/// locked for share until `tx` ends, so a delete waits for it: an open that read the row
/// commits its ticket and `recording.view` before the delete can withdraw it (G-7).
pub(super) async fn catalogued(
    tx: &mut PgConnection,
    id: RecordingId,
    now: DateTime<Utc>,
) -> Result<Option<Catalogued>, Error> {
    let row: Option<CataloguedRow> = sqlx::query_as(&format!(
        "select {CATALOGUED_COLUMNS} from sessions.recordings where {shown} and id = $2 \
         for share",
        shown = shown(),
    ))
    .bind(now)
    .bind(id)
    .fetch_optional(tx)
    .await?;
    row.map(Catalogued::try_from).transpose()
}

/// Deletes the recording's row, and with it the only copy of its sealed key (D-13). Returns
/// the row as it was, or `None` when there was none.
pub(super) async fn delete<'e>(
    db: impl PgExecutor<'e>,
    id: RecordingId,
) -> Result<Option<Catalogued>, Error> {
    let row: Option<CataloguedRow> = sqlx::query_as(&format!(
        "delete from sessions.recordings where id = $1 returning {CATALOGUED_COLUMNS}"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(Catalogued::try_from).transpose()
}
