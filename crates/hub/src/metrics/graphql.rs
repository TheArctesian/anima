//! The metrics GraphQL part: `Subscription.metricsAdded` (K1; API-003, MET-005, D-9), and
//! `enrolAgent` and `revokeAgentToken` (K1, K8 Section 3; MET-012, MET-013).
//!
//! The metric types are served from `model.rs`; the `Device` fields that return them
//! (`readings`, `metrics`, `gpus`) belong to the devices domain. Each mutation writes its
//! change, one audit entry and one event in one transaction; neither entry holds the code.

use agent_core::wire::{CODE_BYTES, EnrolmentCode};
use async_graphql::futures_util::stream::{self, Stream};
use async_graphql::{Context, Error as GraphQlError, ID, Object, Result, Subscription};
use chrono::SecondsFormat;
use serde_json::json;
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::{
    Feed, MetricsAdded,
    ingest::{self, AgentEnrolment},
};
use crate::{
    access::Authenticated,
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool, schema_dependency,
    },
    devices::{self, Device},
    events::{AuditAction, AuditTarget, Severity, Source, Trail, TrailEntry},
    ids::DeviceId,
};

#[derive(Default)]
pub struct Mutation;

#[Object(name = "MetricsMutation")]
impl Mutation {
    /// A one-use code that enrols an agent for the device within 15 minutes; the device's
    /// earlier unused codes stop working (MET-012). Login or browser token only (INT-003).
    #[graphql(guard = "Authenticated")]
    async fn enrol_agent(&self, ctx: &Context<'_>, device_id: ID) -> Result<AgentEnrolment> {
        let device = device_of(ctx, wanted(&device_id)?).await?;
        let trail = Trail::from_context(ctx)?;
        let bytes = ingest::random::<CODE_BYTES>().map_err(internal)?;
        let code = EnrolmentCode::from_random(*bytes);
        let mut tx = begin(ctx).await?;
        let enrolment = ingest::make_code(&mut tx, device.id, code, trail.now)
            .await
            .map_err(ingest_error)?;
        let entry = code_made(&device, &enrolment);
        trail.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        Ok(enrolment)
    }

    /// Ends the device's agent token and its unused codes; the next push or renewal with the
    /// token is refused. The metrics source stays `AGENT` (MET-013, R-092 D-7).
    #[graphql(guard = "Authenticated")]
    async fn revoke_agent_token(&self, ctx: &Context<'_>, device_id: ID) -> Result<Device> {
        let device = device_of(ctx, wanted(&device_id)?).await?;
        let trail = Trail::from_context(ctx)?;
        let mut tx = begin(ctx).await?;
        let had_token = ingest::revoke(&mut tx, device.id)
            .await
            .map_err(ingest_error)?;
        let entry = token_revoked(&device, had_token);
        trail.write(&mut tx, entry).await.map_err(internal)?;
        commit(tx).await?;
        Ok(device)
    }
}

/// The device the mutation is about; `NOT_FOUND` when there is none.
async fn device_of(ctx: &Context<'_>, id: DeviceId) -> Result<Device> {
    devices::get(pool(ctx)?, id).await.map_err(device_error)
}

fn device_error(error: devices::Error) -> GraphQlError {
    match error {
        devices::Error::NotFound(_) => coded("No such device.", ErrorCode::NotFound),
        other => internal(other),
    }
}

/// `NOT_FOUND` for a device deleted after it was read; anything else is internal.
fn ingest_error(error: ingest::Error) -> GraphQlError {
    match error {
        ingest::Error::NoDevice => coded("No such device.", ErrorCode::NotFound),
        other => internal(other),
    }
}

/// The audit entry and event of a new code: when it stops working, never the code (LOG-007).
fn code_made(device: &Device, enrolment: &AgentEnrolment) -> TrailEntry {
    let expires_at = enrolment
        .expiry()
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    TrailEntry {
        source: Source::Metrics,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Made an agent enrolment code for {}.", device.name),
        action: AuditAction::Create,
        target: AuditTarget::Device(device.id),
        change: json!({ "agentEnrolment": { "expiresAt": expires_at } }),
    }
}

/// The audit entry and event of a revoked token.
fn token_revoked(device: &Device, had_token: bool) -> TrailEntry {
    TrailEntry {
        source: Source::Metrics,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Revoked the agent token of {}.", device.name),
        action: AuditAction::Delete,
        target: AuditTarget::Device(device.id),
        change: json!({ "agent": { "tokenRevoked": had_token } }),
    }
}

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
