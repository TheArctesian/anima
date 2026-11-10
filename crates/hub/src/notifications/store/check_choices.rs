//! What each check chooses (NOT-003, NOT-005) and what the decision core remembers (D-16).

use std::collections::HashMap;

use sqlx::{PgConnection, PgExecutor, types::Json};
use uuid::Uuid;

use super::rows::CheckStateRow;
use crate::{
    ids::{ChannelId, CheckId},
    notifications::{
        Error,
        model::{CheckSettings, CheckState, status_text},
    },
};

fn uuids<T: Copy + Into<Uuid>>(ids: &[T]) -> Vec<Uuid> {
    ids.iter().map(|id| (*id).into()).collect()
}

/// Makes `channels` the only channels `check` notifies. The channels must exist.
pub(in crate::notifications) async fn set_check_channels(
    conn: &mut PgConnection,
    check: CheckId,
    channels: &[ChannelId],
) -> Result<(), Error> {
    sqlx::query("DELETE FROM notifications.check_channels WHERE check_id = $1")
        .bind(check)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO notifications.check_channels (check_id, channel_id) \
         SELECT $1, channel FROM unnest($2::uuid[]) AS channel ON CONFLICT DO NOTHING",
    )
    .bind(check)
    .bind(uuids(channels))
    .execute(conn)
    .await?;
    Ok(())
}

/// The channels each of `checks` notifies, by name; a check with none has no entry.
pub(in crate::notifications) async fn check_channels(
    db: impl PgExecutor<'_>,
    checks: &[CheckId],
) -> Result<HashMap<CheckId, Vec<ChannelId>>, Error> {
    let rows: Vec<(CheckId, ChannelId)> = sqlx::query_as(
        "SELECT cc.check_id, cc.channel_id FROM notifications.check_channels cc \
         JOIN notifications.channels c ON c.id = cc.channel_id \
         WHERE cc.check_id = ANY($1) ORDER BY c.name",
    )
    .bind(uuids(checks))
    .fetch_all(db)
    .await?;
    let mut by_check: HashMap<CheckId, Vec<ChannelId>> = HashMap::new();
    for (check, channel) in rows {
        by_check.entry(check).or_default().push(channel);
    }
    Ok(by_check)
}

/// Whether `check` still exists and no longer notifies `channel`: the operator turned the
/// channel off for the check after a notification was queued. A check that is gone has no
/// state row (its foreign key cascades), so this is false for it and its rows still go out.
pub(in crate::notifications) async fn channel_turned_off(
    db: impl PgExecutor<'_>,
    check: CheckId,
    channel: ChannelId,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM notifications.check_state WHERE check_id = $1) \
           AND NOT EXISTS (SELECT 1 FROM notifications.check_channels \
                           WHERE check_id = $1 AND channel_id = $2)",
    )
    .bind(check)
    .bind(channel)
    .fetch_one(db)
    .await?)
}

/// The settings of each of `checks`; a check with no row has no entry (the default applies).
pub(in crate::notifications) async fn check_settings(
    db: impl PgExecutor<'_>,
    checks: &[CheckId],
) -> Result<HashMap<CheckId, CheckSettings>, Error> {
    let rows: Vec<(CheckId, i32)> = sqlx::query_as(
        "SELECT check_id, resend_every_beats FROM notifications.check_settings \
         WHERE check_id = ANY($1)",
    )
    .bind(uuids(checks))
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(check, beats)| {
            let resend_every_beats = u16::try_from(beats).map_err(|_| Error::Unreadable)?;
            Ok((check, CheckSettings { resend_every_beats }))
        })
        .collect()
}

/// Stores the settings of `check`.
pub(in crate::notifications) async fn write_check_settings(
    conn: &mut PgConnection,
    check: CheckId,
    settings: CheckSettings,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO notifications.check_settings (check_id, resend_every_beats) \
         VALUES ($1, $2) \
         ON CONFLICT (check_id) DO UPDATE SET resend_every_beats = EXCLUDED.resend_every_beats",
    )
    .bind(check)
    .bind(i32::from(settings.resend_every_beats))
    .execute(conn)
    .await?;
    Ok(())
}

/// The state of `check`, locked until the caller's transaction ends, so two beats of one check
/// decide one after the other. A check with no state yet gets the empty one.
pub(in crate::notifications) async fn lock_check_state(
    conn: &mut PgConnection,
    check: CheckId,
) -> Result<CheckState, Error> {
    sqlx::query(
        "INSERT INTO notifications.check_state (check_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(check)
    .execute(&mut *conn)
    .await?;
    let row: CheckStateRow = sqlx::query_as(
        "SELECT notified_status, down_beats, expiry_marks FROM notifications.check_state \
         WHERE check_id = $1 FOR UPDATE",
    )
    .bind(check)
    .fetch_one(conn)
    .await?;
    CheckState::try_from(row)
}

/// Stores the state of `check`.
pub(in crate::notifications) async fn write_check_state(
    conn: &mut PgConnection,
    check: CheckId,
    state: &CheckState,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO notifications.check_state (check_id, notified_status, down_beats, \
         expiry_marks) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (check_id) DO UPDATE SET notified_status = EXCLUDED.notified_status, \
         down_beats = EXCLUDED.down_beats, expiry_marks = EXCLUDED.expiry_marks",
    )
    .bind(check)
    .bind(state.notified_status.map(status_text))
    .bind(i32::try_from(state.down_beats).unwrap_or(i32::MAX))
    .bind(Json(&state.expiry_marks))
    .execute(conn)
    .await?;
    Ok(())
}
