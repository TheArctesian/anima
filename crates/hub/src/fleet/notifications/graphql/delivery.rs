//! `Delivery` as the API returns it, and the batched read behind
//! `NotificationChannel.lastDelivery`: one query per batch of channels (API-005, NOT-004).
//!
//! A delivery holds a reason word and a status or reply code, never the target's reply text
//! (CR-14, LOG-007).

use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    ID, SimpleObject,
    dataloader::{DataLoader, Loader},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::enums::{DeliveryReason, NotificationKind};
use crate::{
    fleet::notifications::{Delivery, Error, store},
    ids::ChannelId,
};

#[derive(SimpleObject)]
#[graphql(name = "Delivery")]
pub(in crate::fleet::notifications) struct DeliveryNode {
    id: ID,
    channel_id: Option<ID>,
    channel_name: String,
    kind: NotificationKind,
    at: DateTime<Utc>,
    ok: bool,
    /// Null when ok. The client shows a fixed sentence for each value (CR-14).
    reason: Option<DeliveryReason>,
    /// The HTTP status or SMTP reply code, when the reason has one.
    code: Option<i32>,
}

impl From<Delivery> for DeliveryNode {
    fn from(delivery: Delivery) -> Self {
        Self {
            id: ID(delivery.id.to_string()),
            channel_id: delivery.channel.map(|id| ID(id.to_string())),
            channel_name: delivery.channel_name,
            kind: delivery.kind.into(),
            at: delivery.at,
            ok: delivery.ok,
            reason: delivery.reason.map(Into::into),
            code: delivery.code,
        }
    }
}

/// The newest delivery of each channel that has one.
pub struct LastDeliveries(PgPool);

impl Loader<ChannelId> for LastDeliveries {
    type Value = Delivery;
    type Error = Arc<Error>;

    async fn load(&self, keys: &[ChannelId]) -> Result<HashMap<ChannelId, Delivery>, Self::Error> {
        store::last_deliveries(&self.0, keys)
            .await
            .map_err(Arc::new)
    }
}

/// The loader behind `NotificationChannel.lastDelivery`: one query per batch of channel ids.
#[must_use]
pub fn last_delivery_loader(pool: PgPool) -> DataLoader<LastDeliveries> {
    DataLoader::new(LastDeliveries(pool), tokio::spawn)
}
