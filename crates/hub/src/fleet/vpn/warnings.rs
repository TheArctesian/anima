//! The warnings a picture calls for: keys that expire soon, tagged keys that expire, and nodes
//! that are not in Anima (VPN-010, VPN-011). Pure: the store decides which ones are new.

use chrono::{DateTime, Utc};

use super::model::{KeyState, Picture, VpnNode};
use crate::{fleet::events::Severity, ids::DeviceId};

/// Days before an expiry at which a warning is due, smallest first (VPN-010).
const THRESHOLDS: [(i64, Kind); 4] = [
    (1, Kind::Expiry1),
    (7, Kind::Expiry7),
    (14, Kind::Expiry14),
    (30, Kind::Expiry30),
];
/// The longest node key the warning marks accept (`vpn.warnings.node_key`).
const KEY_MAX: usize = 80;

/// The reason for a warning; one mark for each node, kind and expiry (BD-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Expiry30,
    Expiry14,
    Expiry7,
    Expiry1,
    TaggedExpiry,
    NotInAnima,
}

impl Kind {
    /// The text stored in `vpn.warnings.kind`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Expiry30 => "expiry_30",
            Self::Expiry14 => "expiry_14",
            Self::Expiry7 => "expiry_7",
            Self::Expiry1 => "expiry_1",
            Self::TaggedExpiry => "tagged_expiry",
            Self::NotInAnima => "not_in_anima",
        }
    }
}

/// One warning event, with what its mark is keyed on.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Warning {
    pub key: String,
    pub kind: Kind,
    pub key_expiry: Option<DateTime<Utc>>,
    pub device: Option<(DeviceId, String)>,
    pub severity: Severity,
    pub message: String,
}

/// Everything but the node key, which stays out of logs and panic messages (LOG-007).
impl std::fmt::Debug for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Warning")
            .field("kind", &self.kind)
            .field("key_expiry", &self.key_expiry)
            .field("device", &self.device)
            .field("severity", &self.severity)
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}

/// The warnings this picture calls for; the store decides which ones are new. A node whose
/// key the marks cannot hold gets none, so one odd node cannot stop the others' warnings.
pub(super) fn due(picture: &Picture, now: DateTime<Utc>) -> Vec<Warning> {
    std::iter::once(&picture.hub)
        .chain(&picture.nodes)
        .filter(|node| node.key.starts_with("nodekey:") && node.key.len() <= KEY_MAX)
        .flat_map(|node| [expiry(node, now), tagged(node), outside(node)])
        .flatten()
        .collect()
}

/// An untagged key that expires within 30 days, or has expired: the most urgent threshold
/// only, so a hub that was stopped for a while writes one event, not four.
fn expiry(node: &VpnNode, now: DateTime<Utc>) -> Option<Warning> {
    if !matches!(node.key_state, KeyState::Expiring | KeyState::Expired) {
        return None;
    }
    let expiry = node.key_expiry?;
    let seconds = (expiry - now).num_seconds().max(0);
    let days = (seconds + 86_399) / 86_400;
    let (_, kind) = THRESHOLDS.into_iter().find(|&(limit, _)| days <= limit)?;
    let severity = if days <= 1 {
        Severity::Error
    } else {
        Severity::Warning
    };
    let date = expiry.format("%Y-%m-%d");
    let message = if days == 0 {
        format!("VPN key of {} expired on {date}", node.name)
    } else {
        format!("VPN key of {} expires in {days} days, on {date}", node.name)
    };
    Some(Warning {
        key_expiry: Some(expiry),
        ..warning(node, kind, severity, message)
    })
}

/// A tagged node whose key expires; marked once for the node (BD-4).
fn tagged(node: &VpnNode) -> Option<Warning> {
    (node.key_state == KeyState::TaggedWithExpiry).then(|| {
        let message = format!(
            "VPN node {} has a tag and a key expiry; tagged keys must not expire (VPN-003)",
            node.name
        );
        warning(node, Kind::TaggedExpiry, Severity::Warning, message)
    })
}

/// A node that Anima does not know (VPN-011).
fn outside(node: &VpnNode) -> Option<Warning> {
    (!node.in_anima).then(|| {
        let message = if node.role == super::Role::Router {
            format!(
                "VPN router {} ({}) serves no Anima site",
                node.name, node.address
            )
        } else {
            format!("VPN node {} ({}) is not in Anima", node.name, node.address)
        };
        warning(node, Kind::NotInAnima, Severity::Warning, message)
    })
}

/// A warning about `node` with no expiry in its mark.
fn warning(node: &VpnNode, kind: Kind, severity: Severity, message: String) -> Warning {
    Warning {
        key: node.key.clone(),
        kind,
        key_expiry: None,
        device: node.device.clone(),
        severity,
        message,
    }
}
