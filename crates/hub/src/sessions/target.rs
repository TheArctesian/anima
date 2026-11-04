//! Which addresses a session or an SSH probe may reach (R-032): never the gateway's own
//! machine, a link-local neighbour or a group address. Pure.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

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

/// The rule a probe applies to every address a host resolves to. The hub always uses
/// [`TargetRule::STRICT`]; tests that run an SSH server on the loopback interface use
/// [`TargetRule::LOOPBACK_FOR_TESTS`], which no release build contains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TargetRule {
    loopback: bool,
}

impl TargetRule {
    /// Refuses every address [`is_refused_target`] refuses.
    pub(super) const STRICT: Self = Self { loopback: false };

    /// Lets plain loopback addresses (`127.0.0.0/8`, `::1`) through; every other refused
    /// address stays refused.
    #[cfg(any(test, feature = "test-support"))]
    pub(super) const LOOPBACK_FOR_TESTS: Self = Self { loopback: true };

    /// Whether a probe must not connect to `ip`.
    pub(super) fn refuses(self, ip: IpAddr) -> bool {
        is_refused_target(ip) && !(self.loopback && ip.is_loopback())
    }

    /// The addresses a host resolved to, when the probe may connect to every one; `None` when
    /// any is refused, so a name that also points at the gateway is refused whichever address
    /// would answer first.
    pub(super) fn checked(self, addresses: Vec<SocketAddr>) -> Option<Vec<SocketAddr>> {
        let refused = addresses.iter().any(|address| self.refuses(address.ip()));
        (!refused).then_some(addresses)
    }
}
