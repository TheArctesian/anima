//! The `LocalAPI` status reply as the hub reads it, and the cleaning of the text a node chose.
//!
//! Every field is the node's or the control server's word, so nothing here is trusted: text is
//! cleaned and cut before it is shown, unknown fields are ignored, and Go's `null` lists read as
//! empty (VPN-008).

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, Datelike as _, Utc};
use ipnet::IpNet;
use serde::{Deserialize, Deserializer};

use super::Error;

/// The part of `/localapi/v0/status` the picture reads.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct Status {
    #[serde(rename = "Self")]
    pub own: Peer,
    /// `null` when the node has no peers.
    #[serde(default)]
    pub peer: Option<HashMap<String, Peer>>,
}

/// One node in the status reply: the hub itself or a peer.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct Peer {
    pub public_key: String,
    pub host_name: String,
    #[serde(rename = "DNSName")]
    pub dns_name: String,
    #[serde(rename = "OS")]
    pub os: String,
    /// IPv4 first, then IPv6. Go writes `null` for none.
    #[serde(rename = "TailscaleIPs", default, deserialize_with = "null_is_empty")]
    pub tailscale_ips: Vec<IpAddr>,
    #[serde(default, deserialize_with = "null_is_empty")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "null_is_empty")]
    pub primary_routes: Vec<IpNet>,
    pub online: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub cur_addr: String,
    /// The home DERP region; set for every node, used only while traffic flows.
    #[serde(default)]
    pub relay: String,
    #[expect(
        clippy::struct_field_names,
        reason = "the name is Tailscale's `PeerRelay`, which the spec's path rule reads"
    )]
    #[serde(default)]
    pub peer_relay: String,
    #[serde(default, deserialize_with = "zero_is_none")]
    pub last_seen: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "zero_is_none")]
    pub key_expiry: Option<DateTime<Utc>>,
}

/// The status in `body`.
///
/// # Errors
/// `Unreadable` on any shape it does not know; the reply text is not kept (LOG-007).
pub(super) fn parse(body: &[u8]) -> Result<Status, Error> {
    serde_json::from_slice(body).map_err(|_| Error::Unreadable)
}

/// `crate::text::clean(text, max)`, or "?" when that is empty (VPN-008).
pub(super) fn shown(text: &str, max: usize) -> String {
    let cleaned = crate::text::clean(text, max);
    if cleaned.is_empty() {
        "?".to_owned()
    } else {
        cleaned
    }
}

/// JSON `null` or an absent list is an empty list.
fn null_is_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// Go's zero time (`0001-01-01T00:00:00Z`), `null` or an absent time is none.
fn zero_is_none<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let time = Option::<DateTime<Utc>>::deserialize(deserializer)?;
    Ok(time.filter(|time| time.year() > 1))
}
