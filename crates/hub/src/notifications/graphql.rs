//! The notifications GraphQL part: root `Query` and `Mutation` for notification channels (K1;
//! API-003, NOT-002, NOT-003, SEC-005, LOG-005, LOG-007).
//!
//! `NotificationChannel` and the types below it live in the sub-modules. A mutation writes its
//! change, one audit entry and one event in one transaction. No secret is ever returned,
//! audited, recorded as an event or written to the log; the channel input refuses to quote
//! what it could not read.

mod delivery;
mod enums;
mod form;
mod input;
mod node;
mod trail;
mod write;

use async_graphql::{Context, Error as GraphQlError, ID, Object, Result};

pub use self::delivery::{LastDeliveries, last_delivery_loader};

use self::{
    input::{ChannelShape, Opaque},
    node::ChannelNode,
};
use super::{Channel, Error, store};
use crate::{
    access::Authenticated,
    api_errors::{FieldError, internal, invalid, pool},
};

/// The GraphQL error with its `extensions.code`. A taken name is the operator's to fix;
/// anything else is `INTERNAL` with no detail, the cause kept as the error's source.
fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NameTaken => invalid(&[FieldError::new("name", "Another channel has this name.")]),
        other => internal(other),
    }
}

/// The channel with its settings read, or `None` after logging its id and the fault word
/// (never the stored settings; LOG-007).
fn readable(channel: Channel) -> Option<ChannelNode> {
    let id = channel.id;
    ChannelNode::try_from(channel)
        .inspect_err(|error| {
            tracing::warn!(
                channel = %id,
                fault = error.fault(),
                "a notification channel's stored settings cannot be read; it is left out of the list"
            );
        })
        .ok()
}

#[derive(Default)]
pub struct Query;

#[Object(name = "NotificationsQuery")]
impl Query {
    /// Every notification channel, by name. A channel whose stored settings cannot be read is
    /// left out and its id written to the process log: K1 has no settings value that says
    /// "unreadable", and one bad row must not hide every other channel.
    #[graphql(guard = "Authenticated")]
    async fn notification_channels(&self, ctx: &Context<'_>) -> Result<Vec<ChannelNode>> {
        let channels = store::list_channels(pool(ctx)?, None)
            .await
            .map_err(to_graphql)?;
        Ok(channels.into_iter().filter_map(readable).collect())
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "NotificationsMutation")]
impl Mutation {
    #[graphql(guard = "Authenticated")]
    async fn create_notification_channel(
        &self,
        ctx: &Context<'_>,
        input: Opaque<ChannelShape>,
    ) -> Result<ChannelNode> {
        write::create(ctx, input.0).await
    }

    /// Null secret fields keep the stored secrets, unless the target changes (CR-14).
    #[graphql(guard = "Authenticated")]
    async fn update_notification_channel(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: Opaque<ChannelShape>,
    ) -> Result<ChannelNode> {
        write::update(ctx, &id, input.0).await
    }

    #[graphql(guard = "Authenticated")]
    async fn delete_notification_channel(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        write::delete(ctx, &id).await
    }
}
