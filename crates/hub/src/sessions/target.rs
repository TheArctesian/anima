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
/// embed one: IPv4-compatible `::/96`, IPv4-mapped `::ffff:0:0/96`, SIIT
/// `::ffff:0:0:0/96` and NAT64 `64:ff9b::/96`.
fn embedded_v4(v6: Ipv6Addr) -> Option<Ipv4Addr> {
    let prefix: [u16; 6] = v6.segments()[..6].try_into().ok()?;
    let embeds = matches!(
        prefix,
        [0, 0, 0, 0, 0, 0 | 0xffff] | [0, 0, 0, 0, 0xffff, 0] | [0x64, 0xff9b, 0, 0, 0, 0]
    );
    let [.., high, low] = v6.segments();
    embeds.then(|| Ipv4Addr::from((u32::from(high) << 16) | u32::from(low)))
}

/// Whether `ip` points back at the gateway's own machine or at no single host: loopback,
/// link-local, multicast, broadcast, unspecified or "this network" (`0.0.0.0/8`). An IPv6
/// address that embeds an IPv4 address is also refused when that address is.
pub(super) fn is_refused_target(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_refused_v4(v4),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unicast_link_local()
                || v6.is_multicast()
                || v6.is_unspecified()
                || embedded_v4(v6).is_some_and(is_refused_v4)
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
