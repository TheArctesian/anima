//! K1 `AgentAccount`: an account of an AI agent program and the devices that use it, as keys only
//! (R-NEW-accountgroup, AIS-022).

use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use daemon_core::agent_usage::account::Limit;

use super::AccountGroup;
use crate::fleet::agent_usage::graphql::AgentSource;
use crate::fleet::devices::Device;

/// The hexadecimal digits of a key that the page shows to tell two accounts apart.
const SHORT_KEY: usize = 8;

/// K1 `AgentAccount`.
#[derive(SimpleObject)]
pub struct AgentAccount {
    /// The key of the account: the HMAC-SHA-256 of the vendor's identifier with the salt of this
    /// hub, never the identifier.
    key: String,
    /// The first 8 digits of the key, to tell two accounts of one program apart.
    short_key: String,
    source: AgentSource,
    plan: Option<String>,
    limits: Vec<AgentAccountLimit>,
    reported_at: DateTime<Utc>,
    /// The devices that use the account, newest report first; a deleted device is left out.
    devices: Vec<Device>,
}

/// K1 `AgentAccountLimit`: one limit window of an account.
#[derive(SimpleObject)]
pub struct AgentAccountLimit {
    window: String,
    used_percent: Option<i32>,
    resets_at: Option<DateTime<Utc>>,
    reached: bool,
}

impl AgentAccount {
    /// The account of `group` with the devices of `devices` that use it; `None` when none of
    /// them is left.
    #[must_use]
    pub fn of(group: AccountGroup, devices: &[Device]) -> Option<Self> {
        let used: Vec<Device> = group
            .devices
            .iter()
            .filter_map(|id| devices.iter().find(|device| device.id == *id).cloned())
            .collect();
        (!used.is_empty()).then(|| Self {
            short_key: group.key.chars().take(SHORT_KEY).collect(),
            key: group.key,
            source: group.source.into(),
            plan: group.plan,
            limits: group
                .limits
                .into_iter()
                .map(AgentAccountLimit::from)
                .collect(),
            reported_at: group.reported_at,
            devices: used,
        })
    }
}

impl From<Limit> for AgentAccountLimit {
    fn from(limit: Limit) -> Self {
        Self {
            window: limit.window,
            used_percent: limit.used_percent.map(i32::from),
            resets_at: limit.resets_at,
            reached: limit.reached,
        }
    }
}
