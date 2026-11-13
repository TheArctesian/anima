//! Wake-on-LAN through the site's wake relay (RA-031, VPN-006, D-4): the device's MAC address,
//! sent by `wakeonlan` on another device of the same site over the hub's SSH link (P2-003).
//! `wake` sends whatever the device's state; whether a session start needs it is
//! `startSession`'s decision.
//!
//! Who writes what (LOG-005): [`wake`] writes the event, one per attempt on a known device:
//! Info when the relay sent the packet, Error with a fixed sentence when it did not. The
//! `wakeDevice` mutation, which knows the operator and the client address, writes the audit
//! entry `device.wake` once the packet is sent, as `acceptHostKey` writes its own. No event,
//! audit entry or error holds a host, an address, the MAC address or the relay's output
//! (LOG-007).

pub(super) mod graphql;
mod refusal;
mod relay;
#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;

use crate::{
    credentials,
    devices::{self, Device},
    events::{
        self, ActorKind, AuditAction, AuditTarget, Bus, NewAudit, NewEvent, Severity, Source,
    },
    ids::DeviceId,
    sessions::Actor,
    ssh::{LinkError, link},
    targets::AddressPolicy,
};

/// A Wake-on-LAN packet the relay sent (K1 `WakeResult`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WakeSent {
    /// The time the caller passed to [`wake`].
    pub sent_at: DateTime<Utc>,
    pub relay: DeviceId,
    pub relay_name: String,
}

/// Why no Wake-on-LAN packet was sent. No variant holds a host, an address, the MAC address
/// or the relay's output; the store errors stay generic (API-010, LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("device {0} not found")]
    NotFound(DeviceId),
    #[error("the device has no MAC address")]
    NoMacAddress,
    #[error("the device's site has no wake relay")]
    NoRelay,
    #[error("the device is its own site's wake relay")]
    RelayIsDevice,
    /// The link to the relay did not open, or `wakeonlan` did not run on it.
    #[error("the SSH link to the wake relay failed")]
    Link(#[source] LinkError),
    /// `wakeonlan` ran and ended without exit status 0.
    #[error("wakeonlan on the wake relay did not succeed")]
    NotSent,
    #[error("device store error")]
    Devices(#[source] devices::Error),
    #[error("event store error")]
    Events(#[from] events::Error),
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

/// Sends a Wake-on-LAN packet for `device` from its site's wake relay: the device needs a MAC
/// address, the site a relay that is not the device itself, and the relay a link that opens
/// (SSH on, an accepted host key, a stored credential). The relay runs `wakeonlan
/// <mac>` within 10 seconds; exit status 0 counts as sent. Writes one event for a known
/// device, as the module describes. A failed event write is logged and does not change the
/// result: the packet was sent, or not, either way.
///
/// # Errors
/// `NotFound` for an unknown device; the [`Error`] for the first step that fails.
#[expect(
    clippy::too_many_arguments,
    reason = "the hub contract (K3) fixes the signature of wake"
)]
pub async fn wake(
    db: &PgPool,
    vault: &credentials::Vault,
    policy: &AddressPolicy,
    bus: &Bus,
    device: DeviceId,
    now: DateTime<Utc>,
) -> Result<WakeSent, Error> {
    let device = devices::get(db, device)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => Error::NotFound(device),
            other => Error::Devices(other),
        })?;
    let sent = async {
        let mac = device.mac_address.ok_or(Error::NoMacAddress)?;
        let relay = devices::wake_relay(db, device.site)
            .await
            .map_err(Error::Devices)?;
        let relay = relay::chosen(&device, relay)?;
        let mut link = link(db, vault, policy, &relay, now)
            .await
            .map_err(Error::Link)?;
        relay::send(&mut link, mac).await?;
        Ok(WakeSent {
            sent_at: now,
            relay: relay.id,
            relay_name: relay.name,
        })
    }
    .await;
    if let Err(error) = record(db, bus, event(&device, &sent), now).await {
        tracing::error!(device = %device.id, %error, "the event of a Wake-on-LAN could not be written");
    }
    sent
}

/// The event for `device`'s wake: Info naming the relay, or Error with the fixed sentence.
fn event(device: &Device, sent: &Result<WakeSent, Error>) -> NewEvent {
    let (severity, message) = match sent {
        Ok(sent) => (
            Severity::Info,
            format!(
                "Sent a Wake-on-LAN packet through the wake relay {}.",
                sent.relay_name
            ),
        ),
        Err(error) => (Severity::Error, refusal::sentence(error).to_owned()),
    };
    NewEvent {
        severity,
        source: Source::Session,
        device: Some((device.id, device.name.clone())),
        message,
        // K3 fixes `wake` without its caller, so the event is Hub; the `device.wake` audit
        // entry records the request's kind (LOG-013).
        actor: ActorKind::Hub,
    }
}

async fn record(db: &PgPool, bus: &Bus, event: NewEvent, now: DateTime<Utc>) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    events::record(&mut tx, bus, now, event).await?;
    tx.commit().await?;
    Ok(())
}

/// Writes the audit entry `device.wake` for a packet `by` had `sent` to `device`, stamped
/// `by.at`. The change names the relay; never the MAC address or a host.
///
/// # Errors
/// `Events` or `Database` when the entry cannot be written.
pub(in crate::sessions) async fn audit(
    db: &PgPool,
    by: &Actor,
    device: DeviceId,
    sent: &WakeSent,
) -> Result<(), Error> {
    let entry = NewAudit {
        operator: by.operator.clone(),
        action: AuditAction::Wake,
        target: AuditTarget::Device(device),
        client: by.client,
        change: json!({
            "relayId": sent.relay.to_string(),
            "relayName": sent.relay_name,
        }),
    };
    let mut tx = db.begin().await?;
    events::audit(&mut tx, by.at, entry).await?;
    tx.commit().await?;
    Ok(())
}
