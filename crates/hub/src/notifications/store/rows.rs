//! Rows as the database returns them, and their mapping to the model.

use chrono::{DateTime, Utc};

use crate::{
    ids::{ChannelId, CheckId, DeliveryId},
    notifications::{
        Error,
        model::{
            Channel, ChannelFields, ChannelKind, CheckState, Delivery, ExpiryMark,
            InstallationSettings, NotificationKind, Outbox, OutboxId, status_from_stored,
        },
    },
};

/// Every column of a channel, in the order `ChannelRow` reads them.
pub(super) const CHANNEL_COLUMNS: &str = "id, name, kind, on_by_default, settings, secrets, \
     title_template, body_template, created_at, updated_at";

#[derive(sqlx::FromRow)]
pub(super) struct ChannelRow {
    id: ChannelId,
    name: String,
    kind: String,
    on_by_default: bool,
    settings: serde_json::Value,
    secrets: Option<Vec<u8>>,
    title_template: Option<String>,
    body_template: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ChannelRow> for Channel {
    type Error = Error;

    fn try_from(row: ChannelRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            fields: ChannelFields {
                name: row.name,
                kind: ChannelKind::from_stored(&row.kind).ok_or(Error::Unreadable)?,
                on_by_default: row.on_by_default,
                settings: row.settings,
                secrets: row.secrets,
                title_template: row.title_template,
                body_template: row.body_template,
            },
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct CheckStateRow {
    notified_status: Option<String>,
    down_beats: i32,
    expiry_marks: serde_json::Value,
}

impl TryFrom<CheckStateRow> for CheckState {
    type Error = Error;

    fn try_from(row: CheckStateRow) -> Result<Self, Error> {
        let notified_status = row
            .notified_status
            .map(|text| status_from_stored(&text).ok_or(Error::Unreadable))
            .transpose()?;
        let expiry_marks: Vec<ExpiryMark> =
            serde_json::from_value(row.expiry_marks).map_err(|_| Error::Unreadable)?;
        Ok(Self {
            notified_status,
            down_beats: u32::try_from(row.down_beats).map_err(|_| Error::Unreadable)?,
            expiry_marks,
        })
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct OutboxRow {
    id: OutboxId,
    channel_id: ChannelId,
    check_id: Option<CheckId>,
    kind: String,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
    attempts: i32,
}

impl TryFrom<OutboxRow> for Outbox {
    type Error = Error;

    fn try_from(row: OutboxRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            channel: row.channel_id,
            check: row.check_id,
            kind: NotificationKind::from_stored(&row.kind).ok_or(Error::Unreadable)?,
            payload: row.payload,
            created_at: row.created_at,
            attempts: u32::try_from(row.attempts).map_err(|_| Error::Unreadable)?,
        })
    }
}

/// Every column of an outbox row, in the order `OutboxRow` reads them.
pub(super) const OUTBOX_COLUMNS: &str =
    "id, channel_id, check_id, kind, payload, created_at, attempts";

#[derive(sqlx::FromRow)]
pub(super) struct DeliveryRow {
    id: DeliveryId,
    channel_id: Option<ChannelId>,
    channel_name: String,
    kind: String,
    at: DateTime<Utc>,
    ok: bool,
    message: Option<String>,
}

impl TryFrom<DeliveryRow> for Delivery {
    type Error = Error;

    fn try_from(row: DeliveryRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            channel: row.channel_id,
            channel_name: row.channel_name,
            kind: NotificationKind::from_stored(&row.kind).ok_or(Error::Unreadable)?,
            at: row.at,
            ok: row.ok,
            message: row.message,
        })
    }
}

/// Every column of a delivery, in the order `DeliveryRow` reads them.
pub(super) const DELIVERY_COLUMNS: &str = "id, channel_id, channel_name, kind, at, ok, message";

#[derive(sqlx::FromRow)]
pub(super) struct SettingsRow {
    certificate_days: Vec<i32>,
    title_template: String,
    body_template: String,
}

impl TryFrom<SettingsRow> for InstallationSettings {
    type Error = Error;

    fn try_from(row: SettingsRow) -> Result<Self, Error> {
        let certificate_days = row
            .certificate_days
            .into_iter()
            .map(|days| u16::try_from(days).map_err(|_| Error::Unreadable))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            certificate_days,
            title_template: row.title_template,
            body_template: row.body_template,
        })
    }
}
