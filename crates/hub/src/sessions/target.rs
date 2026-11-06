//! Which addresses a session or an SSH probe may reach (R-032, P2-004): never the gateway's
//! own machine, a link-local neighbour, a group address or a network the hub's address policy
//! refuses. Pure.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use crate::targets::AddressPolicy;

/// Whether an IPv4 address points back at the gateway's own machine or at no single host.
fn is_refused_v4(v4: Ipv4Addr) -> bool {
    v4.is_loopback()
        || v4.is_link_local()
        || v4.is_multicast()
        || v4.is_broadcast()
        || v4.octets()[0] == 0
}

/// The IPv4 address an IPv6 address carries in its last 32 bits, for the prefixes that
/// embed one there: IPv4-compatible `::/96`, IPv4-mapped `::ffff:0:0/96`, SIIT
/// `::ffff:0:0:0/96` and well-known NAT64 `64:ff9b::/96`.
fn embedded_v4(v6: Ipv6Addr) -> Option<Ipv4Addr> {
    let prefix: [u16; 6] = v6.segments()[..6].try_into().ok()?;
    let embeds = matches!(
        prefix,
        [0, 0, 0, 0, 0, 0 | 0xffff] | [0, 0, 0, 0, 0xffff, 0] | [0x64, 0xff9b, 0, 0, 0, 0]
    );
    let [.., high, low] = v6.segments();
    embeds.then(|| Ipv4Addr::from((u32::from(high) << 16) | u32::from(low)))
}

/// The IPv4 address a 6to4 address (`2002::/16`, RFC 3056) carries in bits 16 to 47.
fn six_to_four_v4(v6: Ipv6Addr) -> Option<Ipv4Addr> {
    let [first, high, low, ..] = v6.segments();
    (first == 0x2002).then(|| Ipv4Addr::from((u32::from(high) << 16) | u32::from(low)))
}

/// Prefixes refused outright, whatever they carry: Teredo `2001::/32` (RFC 4380; the client
/// address inside is obfuscated and the relay is chosen by the network) and local-use NAT64
/// `64:ff9b:1::/48` (RFC 8215; each network places the IPv4 address its own way).
fn is_refused_tunnel(v6: Ipv6Addr) -> bool {
    let [first, second, third, ..] = v6.segments();
    matches!((first, second), (0x2001, 0)) || matches!((first, second, third), (0x64, 0xff9b, 1))
}

/// Whether `ip` points back at the gateway's own machine or at no single host: loopback,
/// link-local, multicast, broadcast, unspecified or "this network" (`0.0.0.0/8`). An IPv6
/// address that embeds an IPv4 address (IPv4-compatible, IPv4-mapped, SIIT, NAT64 or 6to4)
/// is refused when that IPv4 address is; Teredo and local-use NAT64 are refused outright.
pub(super) fn is_refused_target(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_refused_v4(v4),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unicast_link_local()
                || v6.is_multicast()
                || v6.is_unspecified()
                || is_refused_tunnel(v6)
                || embedded_v4(v6).is_some_and(is_refused_v4)
                || six_to_four_v4(v6).is_some_and(is_refused_v4)
        }
    }
}

/// The rule every session connection applies to every address a host resolves to: the
/// terminal, the Guacamole tunnel, the SSH probe and `testConnection` (R-032, P2-004). An
/// address is refused when [`is_refused_target`] refuses it or the hub's [`AddressPolicy`]
/// does not permit it: the `ANIMA_PROBE_DENY` networks (R-030) and the policy's own
/// always-refused ranges, such as Tailscale's `100.100.100.0/24` and cloud metadata
/// addresses. The default holds the policy with no deny list; the hub replaces it with its
/// own through [`TargetRule::denying`]. Tests that run an SSH server on the loopback
/// interface add [`TargetRule::reaching_loopback_for_tests`], which no release build contains.
/// Cheap to clone: the policy is shared.
#[derive(Clone, Debug, Default)]
pub(super) struct TargetRule {
    loopback: bool,
    policy: Arc<AddressPolicy>,
}

impl TargetRule {
    /// This rule with `policy` in place of its own; a rule that reaches loopback for tests
    /// keeps doing so, whichever builder runs first.
    pub(super) fn denying(self, policy: AddressPolicy) -> Self {
        Self {
            policy: Arc::new(policy),
            ..self
        }
        .aligned()
    }

    /// This rule letting plain loopback addresses (`127.0.0.0/8`, `::1`) through unless the
    /// policy's deny list holds them; every other refused address stays refused.
    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn reaching_loopback_for_tests(self) -> Self {
        Self {
            loopback: true,
            ..self
        }
        .aligned()
    }

    /// This rule with a policy that lets loopback through exactly when the rule does.
    fn aligned(self) -> Self {
        #[cfg(any(test, feature = "test-support"))]
        if self.loopback {
            let policy = Arc::unwrap_or_clone(self.policy).allowing_loopback();
            return Self {
                loopback: true,
                policy: Arc::new(policy),
            };
        }
        self
    }

    /// Whether a session must not connect to `ip`. The one check every session connection
    /// goes through, so none applies the target rule without the deny list.
    pub(super) fn refuses(&self, ip: IpAddr) -> bool {
        let refused_target = is_refused_target(ip) && !(self.loopback && ip.is_loopback());
        refused_target || !self.policy.permits(ip)
    }

    /// The addresses a host resolved to, when the session may connect to every one; `None`
    /// when any is refused, so a name that also points at the gateway or a denied network is
    /// refused whichever address would answer first.
    pub(super) fn checked(&self, addresses: Vec<SocketAddr>) -> Option<Vec<SocketAddr>> {
        let refused = addresses.iter().any(|address| self.refuses(address.ip()));
        (!refused).then_some(addresses)
    }
}
