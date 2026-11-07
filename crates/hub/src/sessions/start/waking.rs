//! Waking a Down device before its session starts (K4 Section 1, RA-031, VPN-006): when the
//! device is Down, has a MAC address and its site has a wake relay, `start` sends the packet
//! through [`wake`] and the ticket's opening then waits longer.
//!
//! A wake that fails does not stop the start: [`wake`] wrote its Error event with the reason,
//! the session may still connect, and the opening keeps its usual limit. No log or error here
//! holds a host, an address or the MAC address (LOG-007).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    credentials::Vault,
    devices::{self, Device},
    events::Bus,
    sessions::{Error, wake},
    status::Status,
    targets::AddressPolicy,
};

/// What `start` needs to wake the device first, which its own parameters do not give it: the
/// device's status as checks judge it (`Device.status`), and the vault, the hub's one address
/// policy and the bus that [`wake`] uses. Without it `start` sends no packet.
#[derive(Clone, Debug)]
pub struct Waking {
    pub status: Status,
    pub vault: Vault,
    pub policy: AddressPolicy,
    pub bus: Bus,
}

/// Whether a packet for `device` was sent at `now`. None is tried unless the device is Down,
/// has a MAC address and its site has a wake relay.
///
/// # Errors
/// `Devices` when the site's wake relay cannot be read. A failed wake is not an error.
pub(super) async fn sent(
    db: &PgPool,
    device: &Device,
    waking: &Waking,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    if waking.status != Status::Down || device.mac_address.is_none() {
        return Ok(false);
    }
    let relay = devices::wake_relay(db, device.site)
        .await
        .map_err(Error::Devices)?;
    if relay.is_none() {
        return Ok(false);
    }
    let woken = wake(
        db,
        &waking.vault,
        &waking.policy,
        &waking.bus,
        device.id,
        now,
    )
    .await;
    match woken {
        Ok(_) => Ok(true),
        Err(error) => {
            tracing::warn!(device = %device.id, %error, "the session starts without a Wake-on-LAN packet");
            Ok(false)
        }
    }
}
