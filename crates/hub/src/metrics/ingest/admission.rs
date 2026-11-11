//! Whether an agent push or renewal may go on: its bearer token's device, while that device
//! takes its metrics from the agent, with the device's turn (K8 Sections 5 and 6, MET-013,
//! MET-017).

use std::net::IpAddr;

use agent_core::wire::{AgentToken, RefusalCode};
use chrono::{DateTime, Utc};

use super::{Error, credential, routes::AgentState, turns::Turn};
use crate::ids::DeviceId;

/// Why an agent push or renewal stopped short of its answer.
pub(super) enum Stop {
    /// Refused by K8 Section 6, for the device of a valid token when there was one.
    Refused(RefusalCode, Option<DeviceId>),
    /// The hub cannot answer now (`UNAVAILABLE`).
    Failed(Error),
    /// Another request has the turn this one needs (`UNAVAILABLE`, nothing recorded).
    Busy,
}

/// A device whose agent may go on: its token is usable and its metrics come from the agent.
pub(super) struct Admitted {
    pub(super) device: DeviceId,
    pub(super) name: String,
    /// The device's one turn (MET-017), held until the answer.
    pub(super) turn: Turn<DeviceId>,
}

/// The device whose usable token `token` is at `now`, while it takes its metrics from the agent,
/// with its turn. The lookups run under a request turn and under the client `address`'s turn;
/// both end before this returns, so the body that follows arrives holding neither.
pub(super) async fn admit(
    state: &AgentState,
    token: &AgentToken,
    address: Turn<IpAddr>,
    now: DateTime<Utc>,
) -> Result<Admitted, Stop> {
    let (device, name) = {
        let _request = state.turns.request().await.ok_or(Stop::Busy)?;
        let device = holder(state, token, now).await?;
        (device, agent_name(state, device).await?)
    };
    drop(address);
    let turn = state.turns.device(device).ok_or(Stop::Busy)?;
    Ok(Admitted { device, name, turn })
}

/// The device whose usable token `token` is at `now`.
async fn holder(
    state: &AgentState,
    token: &AgentToken,
    now: DateTime<Utc>,
) -> Result<DeviceId, Stop> {
    match credential::authenticate(&state.db, token, now).await {
        Ok(Some(device)) => Ok(device),
        Ok(None) => Err(Stop::Refused(RefusalCode::TokenRefused, None)),
        Err(error) => Err(Stop::Failed(error)),
    }
}

/// The device's name while its metrics come from the agent; `TOKEN_REFUSED` once the operator
/// switched the source away or deleted the device.
async fn agent_name(state: &AgentState, device: DeviceId) -> Result<String, Stop> {
    match credential::agent_name(&state.db, device).await {
        Ok(Some(name)) => Ok(name),
        Ok(None) => Err(Stop::Refused(RefusalCode::TokenRefused, Some(device))),
        Err(error) => Err(Stop::Failed(error)),
    }
}
