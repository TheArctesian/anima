//! `sendTestNotification`: one notification of kind `TEST` sent through a channel at once, on
//! the path every notification takes (NOT-004, D-17, P3-003).
//!
//! It never queues: the send happens while the request waits, within the egress time limit (8 s,
//! from the lookup to the last reply byte read), and its outcome is written as a delivery with
//! one audit entry and one event in one transaction. A failed send is answered as a delivery with
//! its reason, not as an error, so the operator sees what the target did.

use std::sync::Arc;

use async_graphql::{Context, ID, Result};
use chrono::{DateTime, DurationRound, SecondsFormat, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{delivery::DeliveryNode, to_graphql, trail};
use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid, pool, schema_dependency},
    clock::Clock,
    credentials::Vault,
    events::{Bus, ClientAddr, Operator, Trail},
    ids::{ChannelId, DeliveryId},
    notifications::{
        Channel, Delivery, NotificationKind,
        dispatch::{Sender, delivery},
        egress::Egress,
        store,
        template::Values,
    },
    targets::AddressPolicy,
};

/// The values a test notification shows: the same placeholders as a real one, so the operator
/// sees the template filled in.
fn test_values(at: &str) -> Values {
    Values {
        device: Some("Anima".to_owned()),
        check: Some("Test notification".to_owned()),
        status: Some("up".to_owned()),
        previous: Some("up".to_owned()),
        message: Some("This is a test notification from Anima.".to_owned()),
        at: Some(at.to_owned()),
        days: None,
    }
}

/// Who asked for a test notification, and when: the bus its event is announced on, the operator
/// and client address its audit entry names, and the time that stamps both.
pub(super) type Asker = (Bus, (Operator, ClientAddr), DateTime<Utc>);

/// Sends a test notification through channel `id` with `sender`, then writes its delivery,
/// audit entry and event; returns the delivery.
///
/// The send and its record run in a task of their own, so a request dropped while the target
/// has the notification (the client went away) still records it (LOG-005, NOT-004).
///
/// # Errors
/// `NOT_FOUND` when there is no such channel; `INTERNAL` when a read or the write fails, and
/// then nothing is recorded.
pub(super) async fn send_test(
    db: &PgPool,
    sender: &Sender,
    (bus, actor, now): Asker,
    id: ChannelId,
) -> Result<Delivery> {
    let channel = store::list_channels(db, Some(&[id]))
        .await
        .map_err(to_graphql)?
        .pop()
        .ok_or_else(|| coded("No such channel.", ErrorCode::NotFound))?;
    let defaults = store::installation_settings(db).await.map_err(to_graphql)?;
    let now = now
        .duration_trunc(TimeDelta::microseconds(1))
        .unwrap_or(now);
    let (db, sender) = (db.clone(), sender.clone());
    let sent_and_recorded = tokio::spawn(async move {
        let values = test_values(&now.to_rfc3339_opts(SecondsFormat::Secs, true));
        let sent = sender
            .send(&channel, (NotificationKind::Test, &values), &defaults)
            .await;
        let id = DeliveryId::from(Uuid::now_v7());
        let delivery = delivery(id, &channel, (NotificationKind::Test, now), sent);
        let trail = Trail::new(&bus, actor, now);
        record(&db, trail, (&channel, &delivery)).await?;
        Ok(delivery)
    });
    sent_and_recorded.await.map_err(internal)?
}

/// Writes `delivery`, the outcome of a test through `channel`, with its audit entry and event,
/// in one transaction.
async fn record(
    db: &PgPool,
    trail: Trail<'_>,
    (channel, delivery): (&Channel, &Delivery),
) -> Result<()> {
    let mut tx = db.begin().await.map_err(internal)?;
    store::insert_delivery(&mut tx, delivery)
        .await
        .map_err(to_graphql)?;
    trail
        .write(&mut tx, trail::tested(channel, delivery))
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)
}

/// Reads `id`, then sends through the hub's vault and address policy.
pub(super) async fn resolve(ctx: &Context<'_>, id: &ID) -> Result<DeliveryNode> {
    let id: ChannelId = id
        .parse()
        .map_err(|_| invalid(&[FieldError::new("id", "That channel id is not valid.")]))?;
    let sender = Sender::new(
        schema_dependency::<Vault>(ctx)?.clone(),
        Egress::new(schema_dependency::<AddressPolicy>(ctx)?.clone()),
    );
    let asker = (
        schema_dependency::<Bus>(ctx)?.clone(),
        (
            schema_dependency::<Operator>(ctx)?.clone(),
            *schema_dependency::<ClientAddr>(ctx)?,
        ),
        schema_dependency::<Arc<dyn Clock>>(ctx)?.now(),
    );
    send_test(pool(ctx)?, &sender, asker, id)
        .await
        .map(DeliveryNode::from)
}
