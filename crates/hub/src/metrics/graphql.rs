//! The metrics GraphQL part: `Subscription.metricsAdded` (K1; API-003, MET-005, D-9).
//!
//! The metric types are served from `model.rs`; the `Device` fields that return them
//! (`readings`, `metrics`, `gpus`) belong to the devices domain.

use async_graphql::futures_util::stream::{self, Stream};
use async_graphql::{
    Context, Error as GraphQlError, ErrorExtensions as _, ID, Result, Subscription, value,
};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::{Feed, MetricsAdded};
use crate::{access::Authenticated, ids::DeviceId};

/// The text of an `INTERNAL` error: the operator sees no detail.
const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

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
        let feed = ctx.data::<Feed>().map_err(|_| internal())?;
        Ok(added(feed.subscribe(), device))
    }
}

/// The device an id argument names; the error has the shape of every domain's `VALIDATION`.
fn wanted(id: &ID) -> Result<DeviceId> {
    id.parse().map_err(|_| {
        GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
            extensions.set("code", "VALIDATION");
            extensions.set(
                "fields",
                value!([{ "field": "deviceId", "message": "That id is not valid." }]),
            );
        })
    })
}

/// A missing feed is a wiring mistake, never the operator's.
fn internal() -> GraphQlError {
    GraphQlError::new(INTERNAL_MESSAGE).extend_with(|_, extensions| {
        extensions.set("code", "INTERNAL");
    })
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
