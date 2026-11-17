//! Whether a daemon push or renewal may go on: its bearer token's device, while that device
//! takes its metrics from the daemon, with the device's turn (K8 Sections 5 and 6, MET-013,
//! MET-017).

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use daemon_core::wire::{DaemonToken, RefusalCode};

use super::{Error, credential, routes::DaemonState, turns::Turn};
use crate::ids::DeviceId;

/// Why a daemon push or renewal stopped short of its answer.
pub(super) enum Stop {
    /// Refused by K8 Section 6, for the device of a valid token when there was one.
    Refused(RefusalCode, Option<DeviceId>),
    /// The hub cannot answer now (`UNAVAILABLE`).
    Failed(Error),
    /// Another request has the turn this one needs (`UNAVAILABLE`, nothing recorded).
    Busy,
}

/// A device whose daemon may go on: its token is usable and its metrics come from the daemon.
pub(super) struct Admitted {
    pub(super) device: DeviceId,
    pub(super) name: String,
    /// The device's one turn (MET-017), held until the batch or renewal is stored or refused.
    pub(super) turn: Turn<DeviceId>,
}

/// The device whose usable token `token` is at `now`, while it takes its metrics from the daemon,
/// with its turn. The lookups run under a request turn and under the client `address`'s turn;
/// both end before this returns, so the body that follows arrives holding neither. A
/// `Stop::Busy` for the device turn comes after the address turn has ended, and leaves the
/// device turn untaken.
pub(super) async fn admit(
    state: &DaemonState,
    token: &DaemonToken,
    address: Turn<IpAddr>,
    now: DateTime<Utc>,
) -> Result<Admitted, Stop> {
    let (device, name) = {
        let _request = state.turns.request().await.ok_or(Stop::Busy)?;
        let device = holder(state, token, now).await?;
        (device, daemon_name(state, device).await?)
    };
    drop(address);
    let turn = state.turns.device(device).ok_or(Stop::Busy)?;
    Ok(Admitted { device, name, turn })
}

/// The device whose usable token `token` is at `now`.
async fn holder(
    state: &DaemonState,
    token: &DaemonToken,
    now: DateTime<Utc>,
) -> Result<DeviceId, Stop> {
    match credential::authenticate(&state.db, token, now).await {
        Ok(Some(device)) => Ok(device),
        Ok(None) => Err(Stop::Refused(RefusalCode::TokenRefused, None)),
        Err(error) => Err(Stop::Failed(error)),
    }
}

/// The device's name while its metrics come from the daemon; `TOKEN_REFUSED` once the operator
/// switched the source away or deleted the device.
async fn daemon_name(state: &DaemonState, device: DeviceId) -> Result<String, Stop> {
    match credential::daemon_name(&state.db, device).await {
        Ok(Some(name)) => Ok(name),
        Ok(None) => Err(Stop::Refused(RefusalCode::TokenRefused, Some(device))),
        Err(error) => Err(Stop::Failed(error)),
    }
}
