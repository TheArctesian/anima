//! Which addresses a probe may connect to (SSRF protection).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnet::IpNet;

/// The addresses probes may reach.
///
/// The default refuses every address a probe must never touch: loopback, unspecified,
/// `0.0.0.0/8`, link-local (`169.254.0.0/16`, home of cloud metadata, and `fe80::/10`),
/// multicast, broadcast and reserved `240.0.0.0/4`, the AWS IPv6 metadata address, Teredo (`2001::/32`), site-local (`fec0::/10`), and the
/// IPv4-mapped, IPv4-compatible, NAT64 and 6to4 forms of the IPv4 ones. Private ranges
/// (`10/8`, `172.16/12`, `192.168/16`, `100.64/10`, `fc00::/7`) stay allowed, because
/// devices live there; `extra_deny` removes more, such as the hub's own network.
#[derive(Clone, Debug, Default)]
pub struct AddressPolicy {
    extra_deny: Vec<IpNet>,
    allow_loopback: bool,
}

impl AddressPolicy {
    #[must_use]
    pub fn new(extra_deny: Vec<IpNet>) -> Self {
        Self {
            extra_deny,
            allow_loopback: false,
        }
    }

    /// Also allows loopback. For tests that probe local listeners; never use in production.
    #[must_use]
    pub fn allowing_loopback(mut self) -> Self {
        self.allow_loopback = true;
        self
    }

    /// Whether a probe may connect to `ip`.
    #[must_use]
    pub fn permits(&self, ip: IpAddr) -> bool {
        let canonical = canonical(ip);
        if self
            .extra_deny
            .iter()
            .any(|net| net.contains(&ip) || net.contains(&canonical))
        {
            return false;
        }
        if canonical.is_loopback() && self.allow_loopback {
            return true;
        }
        !always_refused(canonical)
    }
}

/// The IPv4 address that an IPv6 form carries, if it carries one.
fn embedded_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let o = ip.octets();
    let v4 = |at: usize| Ipv4Addr::new(o[at], o[at + 1], o[at + 2], o[at + 3]);
    let mapped = o[..10] == [0; 10] && o[10] == 0xff && o[11] == 0xff;
    // ::a.b.c.d, but neither :: nor ::1, which stay IPv6 addresses.
    let compatible = o[..12] == [0; 12] && o[12..] != [0; 4] && o[12..] != [0, 0, 0, 1];
    let nat64 = o[..12] == [0, 0x64, 0xff, 0x9b, 0, 0, 0, 0, 0, 0, 0, 0];
    // ::ffff:0:a.b.c.d (IPv4-translated, SIIT)
    let translated = o[..8] == [0; 8] && o[8..12] == [0xff, 0xff, 0, 0];
    if mapped || compatible || nat64 || translated {
        Some(v4(12))
    } else if o[..6] == [0, 0x64, 0xff, 0x9b, 0, 1] {
        // 64:ff9b:1::/48 (local-use NAT64): the address sits around the reserved octet 8.
        Some(Ipv4Addr::new(o[6], o[7], o[9], o[10]))
    } else if o[0] == 0x20 && o[1] == 0x02 {
        Some(v4(2)) // 2002::/16 (6to4)
    } else {
        None
    }
}

/// The address with embedded IPv4 forms replaced by the IPv4 address.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => embedded_v4(v6).map_or(IpAddr::V6(v6), IpAddr::V4),
        v4 @ IpAddr::V4(_) => v4,
    }
}

fn always_refused(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_unspecified()
                || o[0] == 0
                || v4.is_link_local()
                || v4.is_multicast()
                || o[0] >= 240
        }
        IpAddr::V6(v6) => {
            let [first, second, ..] = v6.segments();
            v6.is_loopback()
                || (first == 0x2001 && second == 0) // 2001::/32 Teredo
                || first & 0xffc0 == 0xfec0 // fec0::/10 site-local
                || v6.is_unspecified()
                || v6.is_multicast()
                || first & 0xffc0 == 0xfe80
                || v6 == Ipv6Addr::new(0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254)
        }
    }
}
