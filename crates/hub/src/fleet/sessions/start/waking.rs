//! Waking a Down device before its session starts (K4 Section 1, RA-031, VPN-006): when the
//! device is Down, has a MAC address and its site has a wake relay, `start` sends the packet
//! through [`wake`] and the ticket's opening then waits longer.
//!
//! A wake that fails does not stop the start: [`wake`] wrote its Error event with the reason,
//! the session may still connect, and the opening keeps its usual limit. No log or error here
//! holds a host, an address or the MAC address (LOG-007).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::fleet::{
    credentials::Vault,
    devices::{self, Device},
    events::{ActorKind, Bus},
    sessions::{Error, wake},
    targets::AddressPolicy,
};

/// What `start` needs to wake the device first, which its own parameters do not give it:
/// whether the device is asleep, and the vault, the hub's one address policy and the bus that
/// [`wake`] uses. Without it `start` sends no packet. Sessions never read a device's status
/// itself; the caller says.
#[derive(Clone, Debug)]
pub struct Waking {
    /// The device is Down as checks judge it (`Device.status`).
    pub asleep: bool,
    pub vault: Vault,
    pub policy: AddressPolicy,
    pub bus: Bus,
}

/// Whether a packet for `device` was sent at `now`, for the start a request of kind `by` asked
/// for; the wake's event records `by` (LOG-013). None is tried unless the device is asleep, has
/// a MAC address and its site has a wake relay.
///
/// # Errors
/// `Devices` when the site's wake relay cannot be read. A failed wake is not an error.
pub(super) async fn sent(
    db: &PgPool,
    (device, by): (&Device, ActorKind),
    waking: &Waking,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    if !waking.asleep || device.mac_address.is_none() {
        return Ok(false);
    }
    let relay = devices::wake_relay(db, device.site)
        .await
        .map_err(Error::Devices)?;
    if relay.is_none() {
        return Ok(false);
    }
    let stores = (db, &waking.vault, &waking.policy, &waking.bus);
    let woken = wake(stores, device.id, by, now).await;
    match woken {
        Ok(_) => Ok(true),
        Err(error) => {
            tracing::warn!(device = %device.id, %error, "the session starts without a Wake-on-LAN packet");
            Ok(false)
        }
    }
}
