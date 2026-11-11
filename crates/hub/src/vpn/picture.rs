//! The picture from one status reply: each node's role, path and key, its Anima device, and
//! the router of each site (VPN-009 to VPN-011). Pure: the reader brings the inputs.

use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr},
};

use chrono::{DateTime, Duration, Utc};
use ipnet::IpNet;

use super::{
    Error,
    broker::BrokerView,
    model::{BrokerState, KeyState, Picture, Role, RouteState, SitePath, VpnNode, VpnPath},
    peers::{Peer, Status, shown},
    waiting,
};
use crate::{
    devices::Device,
    ids::{DeviceId, SiteId},
};

/// How long before its expiry a key is `Expiring` (VPN-010).
const EXPIRING_DAYS: i64 = 30;
/// Reads that keep a node's last known path after its traffic stops (BD-13).
const KEPT_READS: u8 = 3;

/// What one picture is built from. A site is its id and subnet, in site name order.
pub(super) struct Inputs<'a> {
    pub status: &'a Status,
    pub sites: &'a [(SiteId, IpNet)],
    pub devices: &'a [Device],
    /// The router devices with a Tailscale ping check that is not paused.
    pub ping_checked: &'a HashSet<DeviceId>,
    pub now: DateTime<Utc>,
    /// The broker's answer of this read; `broker` is its node list when it is `Ready`.
    pub broker_state: BrokerState,
    pub broker: Option<&'a BrokerView>,
}

/// The picture of `inputs`, fresh, read at `now`.
///
/// # Errors
/// `Unreadable` when the hub's own node has no IPv4 address (it is not on the VPN).
pub(super) fn build(inputs: &Inputs<'_>) -> Result<Picture, Error> {
    let hub = node(&inputs.status.own, inputs).ok_or(Error::Unreadable)?;
    let mut nodes: Vec<VpnNode> = peers(inputs.status)
        .filter_map(|peer| node(peer, inputs))
        .collect();
    nodes.sort_by(|a, b| (&a.name, a.address).cmp(&(&b.name, b.address)));
    let sites: Vec<SitePath> = inputs
        .sites
        .iter()
        .map(|&(site, subnet)| {
            let mut path = site_path(site, subnet, &nodes, inputs.ping_checked);
            path.waiting_router = inputs
                .broker
                .and_then(|view| waiting::router(view, subnet, inputs));
            path
        })
        .collect();
    for router in sites.iter().filter_map(|site| site.router) {
        nodes[router].in_anima = true;
    }
    Ok(Picture {
        read_at: inputs.now,
        fresh: true,
        hub,
        sites,
        nodes,
        broker: inputs.broker_state,
        roster: inputs.broker.cloned(),
    })
}

/// The devices of the peers that serve a site, so the reader reads only their checks.
pub(super) fn router_devices(
    status: &Status,
    sites: &[(SiteId, IpNet)],
    devices: &[Device],
) -> Vec<DeviceId> {
    let mut found: Vec<DeviceId> = peers(status)
        .filter(|peer| role(&peer.tags) == Role::Router)
        .filter(|peer| {
            sites
                .iter()
                .any(|&(_, subnet)| serves(&peer.primary_routes, subnet))
        })
        .filter_map(|peer| device_of(peer, devices).map(|(id, _)| id))
        .collect();
    found.sort_unstable_by_key(|id| uuid::Uuid::from(*id));
    found.dedup();
    found
}

/// BD-13: a node whose path is Unknown in `next` keeps its path from `previous` while
/// `unknown_reads` < 3; the counter goes up by one each such read and to 0 on a known path.
/// A node that is offline in `next` keeps nothing: offline is always Unknown (VPN-009).
pub(super) fn carry(previous: &Picture, mut next: Picture) -> Picture {
    let earlier = |key: &str| {
        std::iter::once(&previous.hub)
            .chain(&previous.nodes)
            .find(|node| node.key == key)
    };
    for node in std::iter::once(&mut next.hub).chain(&mut next.nodes) {
        if node.path != VpnPath::Unknown || !node.online {
            continue;
        }
        let Some(before) = earlier(&node.key) else {
            continue;
        };
        node.unknown_reads = before.unknown_reads.saturating_add(1);
        if before.path != VpnPath::Unknown && before.unknown_reads < KEPT_READS {
            node.path = before.path;
            node.relay_region.clone_from(&before.relay_region);
        }
    }
    next
}

fn peers(status: &Status) -> impl Iterator<Item = &Peer> {
    status.peer.iter().flat_map(|peers| peers.values())
}

/// The node of `peer`, or none when it has no IPv4 address.
fn node(peer: &Peer, inputs: &Inputs<'_>) -> Option<VpnNode> {
    let address = first_ipv4(peer)?;
    let role = role(&peer.tags);
    let device = device_of(peer, inputs.devices);
    let (path, relay_region) = path(peer);
    Some(VpnNode {
        key: peer.public_key.clone(),
        name: name(peer),
        address,
        os: shown(&peer.os, 32),
        role,
        tags: peer.tags.iter().map(|tag| shown(tag, 63)).collect(),
        routes: peer.primary_routes.iter().map(IpNet::trunc).collect(),
        waiting_routes: inputs
            .broker
            .map(|view| waiting::routes(view, address))
            .unwrap_or_default(),
        online: peer.online,
        last_seen: peer.last_seen,
        key_expiry: peer.key_expiry,
        key_state: key_state(&peer.tags, peer.key_expiry, inputs.now),
        // A router is in Anima when it serves a site; `build` sets that after the sites.
        in_anima: role == Role::Hub || (role != Role::Router && device.is_some()),
        device,
        path,
        relay_region,
        unknown_reads: 0,
    })
}

/// The first label of the control server's unique name; the node's own host name only when
/// that is empty (BD-14).
fn name(peer: &Peer) -> String {
    match peer.dns_name.split('.').next() {
        Some(label) if !peer.dns_name.is_empty() => shown(label, 63),
        _ => shown(&peer.host_name, 63),
    }
}

fn first_ipv4(peer: &Peer) -> Option<Ipv4Addr> {
    peer.tailscale_ips.iter().find_map(|ip| match ip {
        IpAddr::V4(v4) => Some(*v4),
        IpAddr::V6(_) => None,
    })
}

/// BD-5: the first role whose tag the node has; no tags is a client.
pub(super) fn role(tags: &[String]) -> Role {
    let has = |wanted: &str| tags.iter().any(|tag| tag == wanted);
    if has("tag:hub") {
        Role::Hub
    } else if has("tag:router") {
        Role::Router
    } else if has("tag:agent") {
        Role::Agent
    } else if tags.is_empty() {
        Role::Client
    } else {
        Role::Other
    }
}

/// VPN-009: the path, and the DERP region when the traffic is relayed.
fn path(peer: &Peer) -> (VpnPath, Option<String>) {
    if !peer.online {
        (VpnPath::Unknown, None)
    } else if !peer.peer_relay.is_empty() {
        (VpnPath::PeerRelay, None)
    } else if peer.active && !peer.cur_addr.is_empty() {
        (VpnPath::Direct, None)
    } else if peer.active {
        let region = (!peer.relay.is_empty()).then(|| shown(&peer.relay, 16));
        (VpnPath::Relayed, region)
    } else {
        (VpnPath::Unknown, None)
    }
}

/// VPN-010: a tagged key must not expire; an untagged one warns 30 days ahead.
pub(super) fn key_state(
    tags: &[String],
    expiry: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> KeyState {
    match expiry {
        Some(_) if !tags.is_empty() => KeyState::TaggedWithExpiry,
        None => KeyState::Never,
        Some(expiry) if expiry <= now => KeyState::Expired,
        Some(expiry) if expiry <= now + Duration::days(EXPIRING_DAYS) => KeyState::Expiring,
        Some(_) => KeyState::Valid,
    }
}

/// BD-14: the device whose address is the node's IPv4, else whose host is the node's unique
/// name, else whose dotless host is that name's first label. The host name the node chose is
/// never used: a node cannot hide behind a device's name.
fn device_of(peer: &Peer, devices: &[Device]) -> Option<(DeviceId, String)> {
    device_by(first_ipv4(peer), &peer.dns_name, devices)
}

/// The device of a node with the IPv4 `address` and the unique name `dns_name` (BD-14).
pub(super) fn device_by(
    address: Option<Ipv4Addr>,
    dns_name: &str,
    devices: &[Device],
) -> Option<(DeviceId, String)> {
    let ip = address.map(IpAddr::V4);
    let dns = unqualified(dns_name);
    let label = dns.split('.').next().unwrap_or_default();
    let by_ip = || {
        devices
            .iter()
            .find(|device| ip.is_some() && device.ip == ip)
    };
    let by_name = || {
        devices
            .iter()
            .find(|device| !dns.is_empty() && unqualified(&device.host) == dns)
    };
    let by_label = || {
        devices.iter().find(|device| {
            !label.is_empty() && !device.host.contains('.') && device.host.to_lowercase() == label
        })
    };
    by_ip()
        .or_else(by_name)
        .or_else(by_label)
        .map(|device| (device.id, device.name.clone()))
}

/// The name in lower case without its trailing dot.
fn unqualified(name: &str) -> String {
    name.strip_suffix('.').unwrap_or(name).to_lowercase()
}

/// BD-6: a router serves a site when one of its primary routes is the site subnet exactly.
fn serves(routes: &[IpNet], subnet: IpNet) -> bool {
    routes.iter().any(|route| route.trunc() == subnet.trunc())
}

/// The first router by name that serves `subnet`, and whether its device has a ping check.
fn site_path(
    site: SiteId,
    subnet: IpNet,
    nodes: &[VpnNode],
    ping_checked: &HashSet<DeviceId>,
) -> SitePath {
    let router = nodes
        .iter()
        .position(|node| node.role == Role::Router && serves(&node.routes, subnet));
    let found = router.map(|index| &nodes[index]);
    let state = match found {
        Some(node) if node.online => RouteState::Serving,
        Some(_) => RouteState::Offline,
        None => RouteState::NoRouter,
    };
    let has_ping_check = found
        .and_then(|node| node.device.as_ref())
        .is_some_and(|(id, _)| ping_checked.contains(id));
    SitePath {
        site,
        router,
        state,
        has_ping_check,
        waiting_router: None,
    }
}
