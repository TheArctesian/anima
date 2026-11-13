//! One device's operating system detection: a link opened for it, `uname -s` run, and an exact
//! `Linux` or `Darwin` stored while the device is still at Other (DEV-025, MET-002). I/O edge.
//!
//! A detection that fails leaves the device as it was: it never fails the round, never raises
//! a device event and is tried again next round. Its log lines hold only the device id and
//! fixed texts, never a host, an address or the device's answer (LOG-007).

use std::sync::Arc;

use super::{
    Context,
    collection::{Failure, within_deadline},
    outputs, uname,
};
use crate::{
    fleet::{
        devices::{self, Device, OperatingSystem},
        metrics::commands::kernel_name,
        ssh::{self, LinkError},
    },
    ids::DeviceId,
};

/// Asks `device` for its operating system and stores it when it is one the hub collects.
pub(super) async fn detect(ctx: Arc<Context>, device: Device) {
    match within_deadline(ask(&ctx, &device)).await {
        Ok(Some(os)) => store(&ctx, device.id, os).await,
        Ok(None) => unknown(device.id),
        Err(error) => unread(device.id, &error),
    }
}

/// Opens a link to `device`, runs `uname -s` and reads its answer; the link is dropped after.
async fn ask(ctx: &Context, device: &Device) -> Result<Option<OperatingSystem>, LinkError> {
    let Context { db, collecting, .. } = ctx;
    let now = ctx.clock.now();
    let mut link = ssh::link(db, &collecting.vault, &collecting.policy, device, now).await?;
    let (fixed, limit) = kernel_name();
    let output = link.run(&fixed, limit).await?;
    Ok(outputs::usable(&output).and_then(uname::operating_system))
}

/// Stores `os` on the device while it is at Other. A failed write leaves it as it was.
async fn store(ctx: &Context, device: DeviceId, os: OperatingSystem) {
    match devices::record_detected_os(&ctx.db, device, os).await {
        Ok(true) => detected(device, os),
        Ok(false) => chosen_before(device),
        Err(error) => unstored(device, &error),
    }
}

fn detected(device: DeviceId, os: OperatingSystem) {
    tracing::info!(
        domain = "metrics",
        device_id = %device,
        os = os.as_str(),
        "the device's operating system was detected"
    );
}

fn chosen_before(device: DeviceId) {
    tracing::debug!(
        domain = "metrics",
        device_id = %device,
        "the device's operating system was set before the detection"
    );
}

fn unstored(device: DeviceId, error: &devices::Error) {
    tracing::warn!(
        domain = "metrics",
        device_id = %device,
        %error,
        "the device's detected operating system cannot be stored"
    );
}

fn unread(device: DeviceId, error: &Failure) {
    tracing::debug!(
        domain = "metrics",
        device_id = %device,
        %error,
        "the device's operating system cannot be read"
    );
}

fn unknown(device: DeviceId) {
    tracing::debug!(
        domain = "metrics",
        device_id = %device,
        "the device's operating system is not one the hub collects"
    );
}
