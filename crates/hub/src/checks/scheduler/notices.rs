//! What the scheduler tells operators: a device's status change and a late check, each as
//! a stored event, the change also as a live signal (LOG-004, LOG-010, CHK-010).

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::{
    attempt::{Context, warn},
    log::{logged, warned},
    schedule,
};
use crate::{
    checks::{self, Due, Missing},
    devices,
    events::{self, ActorKind, NewEvent, Severity, Signal, Source},
    ids::DeviceId,
    status::Status,
};

/// Brings `known` up to date with the device's status; returns whether the device is
/// settled. A change is stored as an event first and published only once stored, so a
/// failed write leaves the device unsettled for the next tick (LOG-004). A deleted device
/// is settled and forgotten.
pub(super) async fn settle(
    ctx: &Context,
    known: &mut HashMap<DeviceId, Status>,
    device: DeviceId,
    now: DateTime<Utc>,
) -> bool {
    let status = match checks::device_status(&ctx.db, device, now).await {
        Ok(status) => status,
        Err(checks::Error::NotFound(Missing::Device(_))) => return forget(known, device),
        Err(error) => return unsettled(device, "the device status cannot be read", &error),
    };
    if known.get(&device) == Some(&status) {
        return true;
    }
    let name = match devices::get(&ctx.db, device).await {
        Ok(found) => found.name,
        Err(devices::Error::NotFound(_)) => return forget(known, device),
        Err(error) => return unsettled(device, "the device cannot be read", &error),
    };
    let event = NewEvent {
        severity: schedule::severity_of(status),
        source: Source::Check,
        device: Some((device, name.clone())),
        message: schedule::status_message(&name, status),
        actor: ActorKind::Hub,
    };
    let written = write_event(ctx, event, now).await;
    if logged(
        Some(device),
        "the status change event cannot be written",
        written,
    )
    .is_none()
    {
        return false;
    }
    ctx.bus.publish(Signal::DeviceStatus {
        device,
        status,
        at: now,
    });
    known.insert(device, status);
    true
}

/// Logs why the device stays unsettled; always false.
fn unsettled(device: DeviceId, what: &'static str, error: &dyn std::fmt::Display) -> bool {
    warned(Some(device), what, error);
    false
}

fn forget(known: &mut HashMap<DeviceId, Status>, device: DeviceId) -> bool {
    known.remove(&device);
    true
}

/// Writes the Error event for a late check (CHK-010); returns whether it was written.
pub(super) async fn report_late(ctx: &Context, check: &Due, now: DateTime<Utc>) -> bool {
    let names = async {
        let name = checks::get(&ctx.db, check.id).await?.name;
        let device = devices::get(&ctx.db, check.device)
            .await
            .map_err(checks::from_devices)?
            .name;
        Ok::<_, checks::Error>((name, device))
    };
    let (name, device) = match names.await {
        Ok(names) => names,
        Err(error) => {
            warn(check, "the late check cannot be read", &error);
            return false;
        }
    };
    let event = NewEvent {
        severity: Severity::Error,
        source: Source::Check,
        device: Some((check.device, device.clone())),
        message: schedule::late_message(&name, &device),
        actor: ActorKind::Hub,
    };
    match write_event(ctx, event, now).await {
        Ok(()) => true,
        Err(error) => {
            warn(check, "the late check event cannot be written", &error);
            false
        }
    }
}

/// Writes one event in its own short transaction.
async fn write_event(
    ctx: &Context,
    event: NewEvent,
    now: DateTime<Utc>,
) -> Result<(), events::Error> {
    let mut tx = ctx.db.begin().await?;
    events::record(&mut tx, &ctx.bus, now, event).await?;
    tx.commit().await?;
    Ok(())
}
