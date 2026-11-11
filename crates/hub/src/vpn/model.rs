//! What the VPN picture is made of: nodes, their paths and keys, and each site's router.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::net::Ipv4Addr;

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use ipnet::IpNet;

use super::broker::BrokerView;
use crate::ids::{DeviceId, SiteId};

/// Whether the hub can reach the broker that makes keys and approves routes (VP-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum BrokerState {
    /// `ANIMA_VPN_BROKER` is not set.
    NotSet,
    /// The socket is not there: the broker unit is not installed on the VPS.
    NotInstalled,
    Ready,
    /// The last call failed or its reply was unreadable.
    Failed,
}

/// What a node is for, from its tags (BD-5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "VpnRole")]
pub enum Role {
    Hub,
    Router,
    Agent,
    Client,
    Other,
}

/// How the hub's traffic reaches a node (VPN-009).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "VpnPath")]
pub enum VpnPath {
    Direct,
    PeerRelay,
    Relayed,
    Unknown,
}

/// Whether a site has a router that serves its subnet (VPN-009, BD-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "SiteRouteState")]
pub enum RouteState {
    Serving,
    Offline,
    NoRouter,
}

/// Where a node's key is in its life (VPN-010).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "KeyState")]
pub enum KeyState {
    Never,
    Valid,
    Expiring,
    Expired,
    TaggedWithExpiry,
}

/// One node of the VPN. Every text came from the node and is cleaned (VPN-008).
#[derive(Clone, Debug, PartialEq)]
pub struct VpnNode {
    /// The node's public key (`nodekey:…`): only for the warning marks, never shown.
    pub key: String,
    pub name: String,
    pub address: Ipv4Addr,
    pub os: String,
    pub role: Role,
    pub tags: Vec<String>,
    pub routes: Vec<IpNet>,
    /// The routes the node offers that Headscale has not approved, from the broker (BD-16).
    pub waiting_routes: Vec<IpNet>,
    pub online: bool,
    pub last_seen: Option<DateTime<Utc>>,
    pub key_expiry: Option<DateTime<Utc>>,
    pub key_state: KeyState,
    /// The Anima device on this node: its id and name, as `events::NewEvent.device`.
    pub device: Option<(DeviceId, String)>,
    pub in_anima: bool,
    pub path: VpnPath,
    pub relay_region: Option<String>,
    /// Reads since the path was last known (BD-13).
    pub unknown_reads: u8,
}

/// How the hub reaches one site (VPN-009).
#[derive(Clone, Debug, PartialEq)]
pub struct SitePath {
    pub site: SiteId,
    /// The index in [`Picture::nodes`] of the router that serves the site subnet.
    pub router: Option<usize>,
    pub state: RouteState,
    pub has_ping_check: bool,
    /// A router of the site that offers the subnet and waits for approval, from the broker
    /// (BD-16). It has no path, and an empty `key`: it gets no warning marks.
    pub waiting_router: Option<VpnNode>,
}

/// The VPN as the hub's node saw it at `read_at` (VPN-005, VPN-008).
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub read_at: DateTime<Utc>,
    /// False when the last read failed; the picture is then the one from `read_at`.
    pub fresh: bool,
    pub hub: VpnNode,
    /// One entry for each site, in site name order.
    pub sites: Vec<SitePath>,
    /// Every peer of the hub, in name order, then address order.
    pub nodes: Vec<VpnNode>,
    pub broker: BrokerState,
    /// The broker's node list and policy sites of this read, when it answered: the mutations
    /// map a site, a node name or a waiting router to the broker's names and ids by it.
    pub(super) roster: Option<BrokerView>,
}
