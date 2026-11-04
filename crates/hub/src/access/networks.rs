//! Which client addresses the permitted networks let in, how long a loaded list is trusted,
//! and which refusals are written to the event log (NET-001, NET-002, D-10, D-11).
//!
//! Pure: the layer passes the networks, the address and `now`.

use std::{collections::HashMap, net::IpAddr, sync::Arc};

use chrono::{DateTime, TimeDelta, Utc};
use ipnet::{IpNet, Ipv4Net};

/// How long a list read from the database is used before it is read again (D-10).
const LIST_KEPT: TimeDelta = TimeDelta::seconds(30);

/// One refusal event per client address in this span (D-11).
const REFUSAL_SPAN: TimeDelta = TimeDelta::minutes(1);

/// Refusal events for all addresses together in [`REFUSAL_SPAN`], so a scan from many
/// addresses cannot flood the event log either. Also bounds the memory the limit uses.
pub(super) const REFUSALS_PER_SPAN: usize = 30;

/// The address an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`) stands for; others unchanged.
#[must_use]
pub(super) const fn canonical(addr: IpAddr) -> IpAddr {
    addr.to_canonical()
}

/// The network without host bits; an IPv4-mapped IPv6 network of prefix 96 or longer becomes
/// the IPv4 network it covers, so it matches the canonical addresses [`permits`] compares.
#[must_use]
pub(super) fn canonical_net(net: IpNet) -> IpNet {
    let net = net.trunc();
    let IpNet::V6(v6) = net else {
        return net;
    };
    let mapped = v6.addr().to_ipv4_mapped();
    match (mapped, v6.prefix_len().checked_sub(96)) {
        (Some(v4), Some(prefix)) => Ipv4Net::new(v4, prefix).map_or(net, IpNet::V4),
        _ => net,
    }
}

/// Whether `addr` is inside one of `networks`. Both sides are compared in canonical form, so
/// `::ffff:10.1.2.3` is inside `10.0.0.0/8` and an IPv4 address is never inside an IPv6
/// network such as `::/0`.
#[must_use]
pub(super) fn permits(networks: &[IpNet], addr: IpAddr) -> bool {
    let addr = canonical(addr);
    networks
        .iter()
        .any(|net| canonical_net(*net).contains(&addr))
}

/// Whether `addr` is a container or proxy address (`172.16.0.0/12`), which is never taken as a
/// client: Docker's bridges, its port proxy and the other containers all use it, so such a
/// peer is not the real client (WP-04 deploy notes).
#[must_use]
pub(super) fn untrusted(addr: IpAddr) -> bool {
    UNTRUSTED.contains(&canonical(addr))
}

/// Whether every address of `net` is [`untrusted`], so permitting it could never let a client
/// in.
#[must_use]
pub(super) fn only_untrusted(net: IpNet) -> bool {
    UNTRUSTED.contains(&canonical_net(net))
}

const UNTRUSTED: IpNet = IpNet::V4(Ipv4Net::new_assert(
    std::net::Ipv4Addr::new(172, 16, 0, 0),
    12,
));

/// A list of permitted networks and when it was read.
#[derive(Clone, Debug)]
pub(super) struct Loaded {
    pub networks: Arc<[IpNet]>,
    pub at: DateTime<Utc>,
}

impl Loaded {
    /// The list, in canonical form, read at `at`.
    pub(super) fn new(networks: impl IntoIterator<Item = IpNet>, at: DateTime<Utc>) -> Self {
        Self {
            networks: networks.into_iter().map(canonical_net).collect(),
            at,
        }
    }

    /// Whether the list may still be used at `now`. A clock that went back makes it stale.
    #[must_use]
    pub(super) fn fresh_at(&self, now: DateTime<Utc>) -> bool {
        now >= self.at && now - self.at < LIST_KEPT
    }
}

/// The refusals written to the event log in the last [`REFUSAL_SPAN`], by client address, and
/// the ones the budget left out.
#[derive(Debug, Default)]
pub(super) struct Refusals {
    written: HashMap<IpAddr, DateTime<Utc>>,
    /// Refusals left out because [`REFUSALS_PER_SPAN`] were written, not yet counted in an
    /// event.
    unrecorded: usize,
    /// When the first of `unrecorded` was left out.
    since: Option<DateTime<Utc>>,
}

impl Refusals {
    /// Whether the refusal of `addr` at `now` is written to the event log, and if so notes
    /// it: not when `addr` was written in the last minute, nor when
    /// [`REFUSALS_PER_SPAN`] addresses were; such a refusal is counted for [`Self::unrecorded`].
    pub(super) fn admit(&mut self, addr: IpAddr, now: DateTime<Utc>) -> bool {
        self.written
            .retain(|_, at| *at <= now && now - *at < REFUSAL_SPAN);
        if self.written.contains_key(&addr) {
            return false;
        }
        if self.written.len() >= REFUSALS_PER_SPAN {
            self.unrecorded += 1;
            self.since.get_or_insert(now);
            return false;
        }
        self.written.insert(addr, now);
        true
    }

    /// How many refusals the budget left out, once [`REFUSAL_SPAN`] has passed since the first
    /// of them, so one event a minute at most reports them (D-11). The count starts again.
    pub(super) fn unrecorded(&mut self, now: DateTime<Utc>) -> Option<usize> {
        let since = self.since?;
        if now >= since && now - since < REFUSAL_SPAN {
            return None;
        }
        self.since = None;
        Some(std::mem::take(&mut self.unrecorded))
    }

    /// How many addresses are being remembered.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.written.len()
    }
}
