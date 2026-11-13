//! The web interfaces GraphQL part: root `Mutation` (K1; WEB-001, WEB-002, WEB-008, WEB-009).
//!
//! The output types are in `types`; `Device.webInterfaces` and
//! `Device.webInterfaceSuggestions` belong to the devices part. Each mutation writes its change,
//! one audit entry and one event in one transaction: if any step fails, nothing is stored.

mod error;
mod open_in;
mod trail;
mod types;

use async_graphql::{Context, ID, Object, Result};

use self::{
    error::to_graphql,
    types::{WebInterfaceFields, parse_id},
};
use super::{Acceptance, WebInterface};
use crate::api_errors::{begin, commit, internal, pool};
use crate::{
    fleet::{
        access::Authenticated,
        devices::{self, Device},
        events::Trail,
    },
    ids::DeviceId,
};

/// The device an interface belongs to, for the event's device name (CR-5). It is read after
/// the change succeeded, so the device exists; a failure here is the hub's, not the operator's.
async fn device_of(ctx: &Context<'_>, id: DeviceId) -> Result<Device> {
    devices::get(pool(ctx)?, id).await.map_err(internal)
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "WebInterfaceMutation")]
impl Mutation {
    #[graphql(guard = "Authenticated")]
    async fn create_web_interface(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        input: WebInterfaceFields,
    ) -> Result<WebInterface> {
        let device: DeviceId = parse_id(&device_id, "deviceId")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let interface = super::create(&mut tx, device, &input.into(), trail.now)
            .await
            .map_err(to_graphql)?;
        let device = device_of(ctx, device).await?;
        trail
            .write(&mut tx, trail::added(&interface, &device))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(interface)
    }

    /// Replaces the interface's fields; a new scheme, host or port drops its pin.
    #[graphql(guard = "Authenticated")]
    async fn update_web_interface(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: WebInterfaceFields,
    ) -> Result<WebInterface> {
        let id = parse_id(&id, "id")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let updated = super::update(&mut tx, id, &input.into())
            .await
            .map_err(to_graphql)?;
        let device = device_of(ctx, updated.after.device).await?;
        trail
            .write(
                &mut tx,
                trail::updated(&updated.before, &updated.after, &device),
            )
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(updated.after)
    }

    #[graphql(guard = "Authenticated")]
    async fn delete_web_interface(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let interface = parse_id(&id, "id")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let device = super::delete(&mut tx, interface)
            .await
            .map_err(to_graphql)?;
        let device = device_of(ctx, device).await?;
        trail
            .write(&mut tx, trail::deleted(interface, &device))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(id)
    }

    /// Makes an interface of an HTTP check's URL; an existing interface at that URL is
    /// returned as it is.
    #[graphql(guard = "Authenticated")]
    async fn accept_web_interface_suggestion(
        &self,
        ctx: &Context<'_>,
        check_id: ID,
    ) -> Result<WebInterface> {
        let check = parse_id(&check_id, "checkId")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let accepted = super::accept_suggestion(pool(ctx)?, &mut tx, check, trail.now)
            .await
            .map_err(to_graphql)?;
        let device = device_of(ctx, accepted.interface.device).await?;
        let entry =
            trail::suggestion_accepted(&accepted.interface, &device, (check, accepted.created));
        trail.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        Ok(accepted.interface)
    }

    #[graphql(guard = "Authenticated")]
    async fn dismiss_web_interface_suggestion(
        &self,
        ctx: &Context<'_>,
        check_id: ID,
    ) -> Result<ID> {
        let check = parse_id(&check_id, "checkId")?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let device = super::dismiss_suggestion(pool(ctx)?, &mut tx, check, trail.now)
            .await
            .map_err(to_graphql)?;
        let device = device_of(ctx, device).await?;
        trail
            .write(&mut tx, trail::suggestion_dismissed(check, &device))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(check_id)
    }

    /// Pins the certificate and writes an audit entry (WEB-008, WEB-009).
    #[graphql(guard = "Authenticated")]
    async fn accept_web_certificate(
        &self,
        ctx: &Context<'_>,
        id: ID,
        fingerprint_sha256: String,
    ) -> Result<WebInterface> {
        let id = parse_id(&id, "id")?;
        let trail = Trail::from_context(ctx)?;
        let acceptance = Acceptance {
            fingerprint: &fingerprint_sha256,
            operator: trail.operator(),
            at: trail.now,
        };
        let mut tx = begin(ctx).await?;
        let interface = super::accept_certificate(&mut tx, id, acceptance)
            .await
            .map_err(to_graphql)?;
        let device = device_of(ctx, interface.device).await?;
        trail
            .write(&mut tx, trail::certificate_accepted(&interface, &device))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        Ok(interface)
    }
}
