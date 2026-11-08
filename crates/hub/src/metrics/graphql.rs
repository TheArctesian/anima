//! The metrics GraphQL part: `Subscription.metricsAdded` (K1; API-003, MET-005, D-9).
//!
//! The metric types are served from `model.rs`; the `Device` fields that return them
//! (`readings`, `metrics`, `gpus`) belong to the devices domain.

use async_graphql::futures_util::stream::{self, Stream};
use async_graphql::{Context, ID, Result, Subscription};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::{Feed, MetricsAdded};
use crate::{
    access::Authenticated,
    api_errors::{FieldError, invalid, schema_dependency},
    ids::DeviceId,
};

#[derive(Default)]
pub struct Subscription;

#[Subscription(name = "MetricsSubscription")]
impl Subscription {
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn metrics_added(
        &self,
        ctx: &Context<'_>,
        device_id: Option<ID>,
    ) -> Result<impl Stream<Item = MetricsAdded>> {
        let device = device_id.as_ref().map(wanted).transpose()?;
        let feed = schema_dependency::<Feed>(ctx)?;
        Ok(added(feed.subscribe(), device))
    }
}

/// The device an id argument names.
fn wanted(id: &ID) -> Result<DeviceId> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new("deviceId", "That id is not valid.")]))
}

/// The collections of `device` (of any device when `None`). A subscriber that fell behind
/// skips what it missed; the stream ends when the feed is closed (D-9).
fn added(
    received: Receiver<MetricsAdded>,
    device: Option<DeviceId>,
) -> impl Stream<Item = MetricsAdded> {
    stream::unfold(received, move |mut received| async move {
        loop {
            match received.recv().await {
                Ok(added)
                    if device.is_none_or(|wanted| added.device_id.0 == wanted.to_string()) =>
                {
                    return Some((added, received));
                }
                Ok(_) | Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    })
}
