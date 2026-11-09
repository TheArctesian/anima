//! Notification channels (NOT-002, NOT-003).

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::rows::{CHANNEL_COLUMNS, ChannelRow};
use crate::{
    ids::ChannelId,
    notifications::{
        Error,
        model::{Channel, ChannelFields},
    },
};

/// Stores a new channel. `NameTaken` when another channel has the name.
pub(in crate::notifications) async fn insert_channel(
    conn: &mut PgConnection,
    id: ChannelId,
    fields: &ChannelFields,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO notifications.channels \
         (id, name, kind, on_by_default, settings, secrets, title_template, body_template, \
          created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)",
    )
    .bind(id)
    .bind(&fields.name)
    .bind(fields.kind.as_str())
    .bind(fields.on_by_default)
    .bind(&fields.settings)
    .bind(&fields.secrets)
    .bind(&fields.title_template)
    .bind(&fields.body_template)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(())
}

/// Replaces every field of a channel; false when there is no such channel. `NameTaken` when
/// another channel has the name.
pub(in crate::notifications) async fn update_channel(
    conn: &mut PgConnection,
    id: ChannelId,
    fields: &ChannelFields,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let done = sqlx::query(
        "UPDATE notifications.channels SET name = $2, kind = $3, on_by_default = $4, \
         settings = $5, secrets = $6, title_template = $7, body_template = $8, updated_at = $9 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&fields.name)
    .bind(fields.kind.as_str())
    .bind(fields.on_by_default)
    .bind(&fields.settings)
    .bind(&fields.secrets)
    .bind(&fields.title_template)
    .bind(&fields.body_template)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Deletes a channel, its check links and its queued notifications; its deliveries stay.
/// False when there is no such channel.
pub(in crate::notifications) async fn delete_channel(
    conn: &mut PgConnection,
    id: ChannelId,
) -> Result<bool, Error> {
    let done = sqlx::query("DELETE FROM notifications.channels WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// The channel with `id`, locked until the caller's transaction ends, so an update or delete
/// reads, checks and writes the row no other writer changes meanwhile; `None` when there is
/// no such channel.
pub(in crate::notifications) async fn channel_for_update(
    conn: &mut PgConnection,
    id: ChannelId,
) -> Result<Option<Channel>, Error> {
    let row: Option<ChannelRow> = sqlx::query_as(&format!(
        "SELECT {CHANNEL_COLUMNS} FROM notifications.channels WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(Channel::try_from).transpose()
}

/// The channels with `ids`, or every channel when `ids` is `None`, by name; an id with no
/// channel has no entry.
pub(in crate::notifications) async fn list_channels(
    db: impl PgExecutor<'_>,
    ids: Option<&[ChannelId]>,
) -> Result<Vec<Channel>, Error> {
    let rows: Vec<ChannelRow> = sqlx::query_as(&format!(
        "SELECT {CHANNEL_COLUMNS} FROM notifications.channels \
         WHERE $1::uuid[] IS NULL OR id = ANY($1) ORDER BY name"
    ))
    .bind(ids.map(|ids| {
        ids.iter()
            .map(|id| uuid::Uuid::from(*id))
            .collect::<Vec<_>>()
    }))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Channel::try_from).collect()
}

/// The ids of the channels a new check notifies (NOT-003), by name.
pub(in crate::notifications) async fn on_by_default(
    db: impl PgExecutor<'_>,
) -> Result<Vec<ChannelId>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM notifications.channels WHERE on_by_default ORDER BY name",
    )
    .fetch_all(db)
    .await?)
}
