//! The compute GraphQL part: `Query.devicePolicies` and `Mutation.setDevicePolicy` (K1; waits
//! for R-NEW-devicepolicy; CMP-002, CAP-010 to CAP-012). The mutation writes the policy, one
//! audit entry and one event in one transaction. It sets no switch: the one switch "takes
//! work" stays at the machine and in `setDeviceWork` (USE-004, OD-C8, R-165, R-270).

mod types;

use std::sync::Arc;

use async_graphql::{Context, ID, Object, Result};
use serde_json::json;

pub use types::{
    DevicePolicyInput, DevicePolicyView, ModelWorkStop, QuietHoursInput, QuietHoursView,
};

use super::policy::{self, DevicePolicy, StoreError};
use crate::{
    api_errors::{
        ErrorCode, FieldError, begin, coded, commit, internal, invalid, pool, schema_dependency,
    },
    clock::Clock,
    fleet::{
        access::Authenticated,
        devices::{self, Device},
        events::{AuditAction, AuditTarget, Severity, Source, Trail, TrailEntry},
    },
    ids::DeviceId,
};

#[derive(Default)]
pub struct Query;

#[Object(name = "ComputeQuery")]
impl Query {
    /// The device policy of each device, in device-name order, with whether it takes model
    /// work now and why not (CAP-010 to CAP-012).
    #[graphql(guard = "Authenticated")]
    async fn device_policies(&self, ctx: &Context<'_>) -> Result<Vec<DevicePolicyView>> {
        let db = pool(ctx)?;
        let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
        let mut found = devices::list(db).await.map_err(internal)?;
        found.sort_by(|one, other| {
            (one.name.as_str(), uuid::Uuid::from(one.id))
                .cmp(&(other.name.as_str(), uuid::Uuid::from(other.id)))
        });
        let ids: Vec<DeviceId> = found.iter().map(|device| device.id).collect();
        let statuses = policy::statuses(db, &ids, now).await.map_err(internal)?;
        Ok(found
            .into_iter()
            .zip(statuses)
            .map(|(device, status)| DevicePolicyView { device, status })
            .collect())
    }
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "ComputeMutation")]
impl Mutation {
    /// Replaces the device policy of a device. A value out of range is refused with
    /// `VALIDATION` on its field; an unknown device with `NOT_FOUND`.
    #[graphql(guard = "Authenticated")]
    async fn set_device_policy(
        &self,
        ctx: &Context<'_>,
        device_id: ID,
        input: DevicePolicyInput,
    ) -> Result<DevicePolicyView> {
        let wanted = policy_of(&input)?;
        let device = device_of(ctx, &device_id).await?;
        let trail = Trail::from_context(ctx)?;
        let now = trail.now;
        let mut tx = begin(ctx).await?;
        policy::set_policy(&mut tx, device.id, &wanted, trail.now)
            .await
            .map_err(store_error)?;
        trail
            .write(&mut tx, changed(&device, &input))
            .await
            .map_err(internal)?;
        commit(tx).await?;
        let db = pool(ctx)?;
        let mut statuses = policy::statuses(db, &[device.id], now)
            .await
            .map_err(internal)?;
        let status = statuses
            .pop()
            .ok_or_else(|| coded("No such device.", ErrorCode::NotFound))?;
        Ok(DevicePolicyView { device, status })
    }
}

fn policy_of(input: &DevicePolicyInput) -> Result<DevicePolicy> {
    input.policy().map_err(|error| invalid(&[error]))
}

async fn device_of(ctx: &Context<'_>, id: &ID) -> Result<Device> {
    let id: DeviceId = id
        .parse()
        .map_err(|_| invalid(&[FieldError::new("deviceId", "That id is not valid.")]))?;
    devices::get(pool(ctx)?, id)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => coded("No such device.", ErrorCode::NotFound),
            other => internal(other),
        })
}

fn store_error(error: StoreError) -> async_graphql::Error {
    match error {
        StoreError::NoDevice => coded("No such device.", ErrorCode::NotFound),
        other => internal(other),
    }
}

/// The audit entry and event of a changed policy: the new values, no secret.
fn changed(device: &Device, input: &DevicePolicyInput) -> TrailEntry {
    let hours = input.quiet_hours.as_ref().map(
        |hours| json!({ "start": hours.start, "end": hours.end, "timeZone": hours.time_zone }),
    );
    TrailEntry {
        source: Source::Device,
        severity: Severity::Info,
        device: Some((device.id, device.name.clone())),
        message: format!("Changed the device policy of {}.", device.name),
        action: AuditAction::Update,
        target: AuditTarget::Device(device.id),
        change: json!({ "devicePolicy": {
            "memorySharePercent": input.memory_share_percent,
            "diskSharePercent": input.disk_share_percent,
            "quietHours": hours,
            "mainsOnly": input.mains_only,
            "idleOnly": input.idle_only,
        } }),
    }
}
