//! SNMP secrets whose device host changed after they were entered (SEC-010): the attempt sends
//! nothing, and the hub removes the secrets and writes one Warning event that tells the
//! operator to enter them again.

use chrono::{DateTime, Utc};

use super::{
    attempt::{Context, warn},
    log::Fault,
    schedule,
};
use crate::{
    checks::{Due, store},
    devices::Device,
    events::{self, NewEvent, Severity, Source},
};

/// Removes the check's SNMP secrets when they were entered for another host than `host`, the
/// device's host now, with the event in the same transaction. Nothing is written when the
/// secrets were entered again for `host` meanwhile. A failed write is logged; the next attempt
/// tries again, and sends nothing either way.
pub(super) async fn forget_secrets(
    ctx: &Context,
    (due, device): (&Due, &Device),
    host: &str,
    now: DateTime<Utc>,
) {
    if let Err(error) = forget(ctx, (due, device), host, now).await {
        warn(
            due,
            "the SNMP secrets of a moved device cannot be removed",
            &error,
        );
    }
}

async fn forget(
    ctx: &Context,
    (due, device): (&Due, &Device),
    host: &str,
    now: DateTime<Utc>,
) -> Result<(), Fault> {
    let mut tx = ctx.db.begin().await?;
    let vault = &ctx.probing.vault;
    let Some(check) = store::forget_snmp_secrets(&mut tx, vault, due.id, host).await? else {
        return Ok(());
    };
    let event = NewEvent {
        severity: Severity::Warning,
        source: Source::Check,
        device: Some((device.id, device.name.clone())),
        message: schedule::moved_secrets_message(&check, &device.name),
    };
    events::record(&mut tx, &ctx.bus, now, event).await?;
    tx.commit().await?;
    Ok(())
}
