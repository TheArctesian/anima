//! The checks GraphQL part: root `Query`, `Mutation` and `Subscription` (K1; API-003, API-005,
//! API-006, CHK-005, CHK-007, LOG-005, LOG-007).
//!
//! The `Check` type and the types below it live in the sub-modules. A mutation writes its
//! change, one audit entry and one event in one transaction. No secret is ever returned,
//! audited, recorded as an event or written to the log; the inputs that can carry one refuse
//! to quote what they could not read.

mod check;
mod config_view;
mod enums;
mod error;
mod form;
mod heartbeat;
mod input;
mod live;
mod opaque;
mod trail;
mod write;

use async_graphql::{Context, ID, Object, Result, Subscription, futures_util::stream::Stream};

use self::{
    error::{invalid, schema_dependency, to_graphql},
    input::{CreateShape, UpdateShape},
    live::{DeviceStatusChange, HeartbeatAdded, device_status_changes, heartbeats_added},
    opaque::Opaque,
    write::pool,
};
use super::{Check, Error, FieldError};
use crate::{
    access::Authenticated,
    events::Bus,
    ids::{CheckId, DeviceId},
};

/// The id an optional subscription argument names.
fn wanted<T: std::str::FromStr>(field: &'static str, id: Option<&ID>) -> Result<Option<T>> {
    id.map(|id| {
        id.parse()
            .map_err(|_| invalid(&[FieldError::new(field, "That id is not valid.")]))
    })
    .transpose()
}

#[derive(Default)]
pub struct Query;

#[Object]
impl Query {
    #[graphql(guard = "Authenticated")]
    async fn check(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Check>> {
        let id: CheckId = id
            .parse()
            .map_err(|_| invalid(&[FieldError::new("id", "That check id is not valid.")]))?;
        match super::get(pool(ctx)?, id).await {
            Ok(check) => Ok(Some(check)),
            Err(Error::NotFound(_)) => Ok(None),
            Err(error) => Err(to_graphql(error)),
        }
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object]
impl Mutation {
    #[graphql(guard = "Authenticated")]
    async fn create_check(&self, ctx: &Context<'_>, input: Opaque<CreateShape>) -> Result<Check> {
        write::create(ctx, input.0).await
    }

    /// An HTTP secret left empty keeps the stored one. Sending `config: { push: true }` gives
    /// a PUSH check a new token.
    #[graphql(guard = "Authenticated")]
    async fn update_check(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: Opaque<UpdateShape>,
    ) -> Result<Check> {
        write::update(ctx, &id, input.0).await
    }

    #[graphql(guard = "Authenticated")]
    async fn delete_check(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        write::delete(ctx, &id).await
    }

    #[graphql(guard = "Authenticated")]
    async fn set_check_paused(&self, ctx: &Context<'_>, id: ID, paused: bool) -> Result<Check> {
        write::set_paused(ctx, &id, paused).await
    }
}

#[derive(Default)]
pub struct Subscription;

#[Subscription]
impl Subscription {
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn device_status_changed(
        &self,
        ctx: &Context<'_>,
        device_id: Option<ID>,
    ) -> Result<impl Stream<Item = DeviceStatusChange>> {
        let device: Option<DeviceId> = wanted("deviceId", device_id.as_ref())?;
        let bus = schema_dependency::<Bus>(ctx)?;
        Ok(device_status_changes(bus.subscribe(), device))
    }

    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "the Subscription macro requires async resolvers"
    )]
    #[graphql(guard = "Authenticated")]
    async fn heartbeat_added(
        &self,
        ctx: &Context<'_>,
        check_id: Option<ID>,
        device_id: Option<ID>,
    ) -> Result<impl Stream<Item = HeartbeatAdded>> {
        let check: Option<CheckId> = wanted("checkId", check_id.as_ref())?;
        let device: Option<DeviceId> = wanted("deviceId", device_id.as_ref())?;
        let bus = schema_dependency::<Bus>(ctx)?;
        Ok(heartbeats_added(bus.subscribe(), (check, device)))
    }
}
