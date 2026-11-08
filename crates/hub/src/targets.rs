//! Which addresses the hub may connect to: probes, sessions, metrics and every new outbound
//! connection (SSRF protection, R-030, R-032, P3-003). The one target rule, and the one
//! resolver that applies it before a connection.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use ipnet::IpNet;

/// The addresses the hub may connect to.
///
/// The default refuses every address the hub must never touch: loopback, unspecified,
/// `0.0.0.0/8`, link-local (`169.254.0.0/16`, home of cloud metadata, and `fe80::/10`),
/// multicast, broadcast and reserved `240.0.0.0/4`, the AWS IPv6 metadata address, Teredo
/// (`2001::/32`), site-local (`fec0::/10`), Tailscale's `100.100.100.0/24`, the IETF protocol
/// assignments `192.0.0.0/24`, Azure's `168.63.129.16`, all of the local-use NAT64
/// `64:ff9b:1::/48`, and the IPv4-mapped, IPv4-compatible, NAT64 and 6to4 forms of the IPv4
/// ones. Private ranges (`10/8`, `172.16/12`, `192.168/16`, `100.64/10`, `fc00::/7`) stay
/// allowed, because devices live there; `extra_deny` removes more, such as the hub's own
/// network.
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

    /// Also allows plain loopback (`127.0.0.0/8` and `::1` as written, not an IPv6 form that
    /// carries a loopback address), for the tests that reach local listeners only. The deny
    /// list still wins.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub(crate) fn allowing_loopback(mut self) -> Self {
        self.allow_loopback = true;
        self
    }

    /// Whether [`AddressPolicy::allowing_loopback`] made this policy, so a policy that
    /// replaces it can keep the test switch.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub(crate) const fn allows_loopback(&self) -> bool {
        self.allow_loopback
    }

    /// Whether the hub may connect to `ip`.
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
        if self.allow_loopback && ip.is_loopback() {
            return true;
        }
        !always_refused(canonical) && !local_use_nat64(ip)
    }
}

/// Why a host gives no address to connect to. Holds no host name or address (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TargetError {
    #[error("the host name does not resolve")]
    Unresolved,
    #[error("the host resolves to an address the hub may not reach")]
    Refused,
}

/// Resolves `host` once (see [`lookup`] for the forms it takes) and returns every address
/// it resolves to, each in its canonical form (an IPv4-mapped address is the IPv4 address it
/// is) with `port`, when `policy` permits every one. A name that also points at a refused
/// address is refused whichever address would answer first (D-1, R-032). The caller connects
/// only to a returned address and never resolves the name again, so a second lookup cannot
/// point the connection somewhere else.
///
/// Check probes share [`lookup`] but keep their own rule, which drops the refused addresses
/// and keeps the rest (`checks::probe::resolve`, CHK-001).
///
/// # Errors
/// [`TargetError::Unresolved`] when the name has no address, [`TargetError::Refused`] when
/// `policy` refuses any of them.
pub async fn resolve_checked(
    host: &str,
    port: u16,
    policy: &AddressPolicy,
) -> Result<Vec<SocketAddr>, TargetError> {
    let addresses = lookup(host, &HashMap::new())
        .await?
        .into_iter()
        .map(|ip| SocketAddr::new(ip.to_canonical(), port))
        .collect();
    every_permitted(policy, addresses)
}

/// The addresses `host` names, from one lookup, as resolved: unchecked, so every caller
/// checks each one against the policy before it connects. `host` is a name, an IP literal,
/// or an IPv6 literal in exactly one pair of brackets (as `Url::host_str` writes it); any
/// other bracketed text is unresolved and never reaches DNS. A name in `overrides` (lower-case
/// keys) takes its addresses from there instead of DNS, for the checks tests.
///
/// # Errors
/// [`TargetError::Unresolved`] when `host` names no address.
pub(crate) async fn lookup(
    host: &str,
    overrides: &HashMap<String, Vec<IpAddr>>,
) -> Result<Vec<IpAddr>, TargetError> {
    if let Some(inside) = host.strip_prefix('[') {
        let ip = inside
            .strip_suffix(']')
            .and_then(|literal| literal.parse::<Ipv6Addr>().ok())
            .ok_or(TargetError::Unresolved)?;
        return Ok(vec![IpAddr::V6(ip)]);
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![ip]);
    }
    let found: Vec<IpAddr> = match overrides.get(&host.to_ascii_lowercase()) {
        Some(ips) => ips.clone(),
        None => tokio::net::lookup_host((host, 0))
            .await
            .map_err(|_| TargetError::Unresolved)?
            .map(|address| address.ip())
            .collect(),
    };
    if found.is_empty() {
        Err(TargetError::Unresolved)
    } else {
        Ok(found)
    }
}

/// `addresses` when there is at least one and `policy` permits every one.
fn every_permitted(
    policy: &AddressPolicy,
    addresses: Vec<SocketAddr>,
) -> Result<Vec<SocketAddr>, TargetError> {
    if addresses.is_empty() {
        return Err(TargetError::Unresolved);
    }
    if addresses
        .iter()
        .any(|address| !policy.permits(address.ip()))
    {
        return Err(TargetError::Refused);
    }
    Ok(addresses)
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

/// `64:ff9b:1::/48`, whatever it embeds: the prefix is local-use, so a translator may place
/// an IPv4 address in it by a rule this policy cannot know (CHK-012).
fn local_use_nat64(ip: IpAddr) -> bool {
    matches!(ip, IpAddr::V6(v6) if v6.segments()[..3] == [0x64, 0xff9b, 1])
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
                || o[..3] == [100, 100, 100] // Tailscale 100.100.100.0/24 (quad-100)
                || o[..3] == [192, 0, 0] // 192.0.0.0/24 IETF protocol assignments
                || v4 == Ipv4Addr::new(168, 63, 129, 16) // Azure wire server
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

#[cfg(test)]
mod tests;
