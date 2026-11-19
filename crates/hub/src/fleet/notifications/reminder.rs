//! A reminder of the engine in the outbox (REM-008, REM-010; K-E6 Section 5): the same
//! dispatcher, channels and egress rules as a device status notification. The engine words the
//! title and the body; no template of a channel is read.

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{
    Channel, ChannelKind, Error, NewOutbox, NotificationKind, OutboxId,
    channels::{Secrets, Settings},
    sealing, store,
};
use crate::{fleet::credentials::Vault, ids::ChannelId};

/// The shortest topic of an ntfy channel that gets a reminder: a topic is the only thing that
/// keeps a stranger from reading it, so it must be random (REM-008).
pub const RANDOM_TOPIC_MIN: usize = 16;

/// One reminder to send: the firing it is for, and the words for the lock screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reminder {
    pub firing: Uuid,
    pub title: String,
    pub body: String,
}

/// What became of one reminder at the outbox.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Queued {
    /// Rows queued, one for each channel.
    pub rows: u32,
    /// ntfy channels that got no row because their topic is shorter than 16 characters.
    pub topic_not_random: u32,
}

/// Whether `channel` is an ntfy channel whose topic can be read and is too short. A channel
/// whose settings or secrets cannot be read is queued: the dispatcher records why it failed.
fn topic_too_short(vault: &Vault, channel: &Channel) -> bool {
    let fields = &channel.fields;
    if fields.kind != ChannelKind::Ntfy {
        return false;
    }
    let Ok(settings) = Settings::from_stored(fields.kind, &fields.settings) else {
        return false;
    };
    let opened = fields
        .secrets
        .as_deref()
        .map(|sealed| sealing::open(vault, channel.id, &settings, sealed));
    match opened {
        Some(Ok(Secrets::Ntfy(secrets))) => secrets.topic.expose().len() < RANDOM_TOPIC_MIN,
        _ => false,
    }
}

/// Queues `reminder` to `channels`, or to each channel that is on by default when `channels`
/// is empty, in the caller's transaction, due at `now`. A channel that is gone gets no row.
///
/// # Errors
/// `Database` when a read or a write fails.
pub async fn queue_reminder(
    conn: &mut PgConnection,
    vault: &Vault,
    channels: &[ChannelId],
    (reminder, now): (&Reminder, DateTime<Utc>),
) -> Result<Queued, Error> {
    let ids = if channels.is_empty() {
        store::on_by_default(&mut *conn).await?
    } else {
        channels.to_vec()
    };
    let mut queued = Queued::default();
    for channel in store::list_channels(&mut *conn, Some(&ids)).await? {
        if topic_too_short(vault, &channel) {
            queued.topic_not_random += 1;
            continue;
        }
        let row = NewOutbox {
            id: OutboxId::from(Uuid::now_v7()),
            channel: channel.id,
            check: None,
            kind: NotificationKind::Reminder,
            payload: json!({
                "title": reminder.title,
                "body": reminder.body,
                "firing": reminder.firing.to_string(),
            }),
        };
        store::insert_outbox(conn, &row, now).await?;
        queued.rows += 1;
    }
    Ok(queued)
}

/// Whether each of `ids` is a channel of the hub.
///
/// # Errors
/// `Database`.
pub async fn channels_exist(db: &sqlx::PgPool, ids: &[ChannelId]) -> Result<bool, Error> {
    if ids.is_empty() {
        return Ok(true);
    }
    let found = store::list_channels(db, Some(ids)).await?;
    Ok(ids
        .iter()
        .all(|id| found.iter().any(|channel| channel.id == *id)))
}
