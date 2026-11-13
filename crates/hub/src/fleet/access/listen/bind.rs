//! Which addresses the hub may listen on (NET-003, SEC-008, D-13, R-021 H-1b), and when plain
//! HTTP is allowed (SEC-001). Pure: the caller passes the address.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::Error;

/// Whether the hub may listen on `addr`, with TLS when `tls` is true.
///
/// Accepted: loopback, RFC 1918 (`10/8`, `172.16/12`, `192.168/16`), the VPN range
/// `100.64/10` and IPv6 unique-local `fc00::/7`; an IPv4-mapped IPv6 address counts as its
/// IPv4 address. Refused: the unspecified address (`0.0.0.0`, `::`) and every other address.
/// Without TLS only loopback is accepted.
///
/// # Errors
///
/// `Error::Unspecified`, `Error::Public` or `Error::TlsRequired`.
pub(super) fn check_bind(addr: IpAddr, tls: bool) -> Result<(), Error> {
    let addr = addr.to_canonical();
    if addr.is_unspecified() {
        return Err(Error::Unspecified(addr));
    }
    let loopback = addr.is_loopback();
    if !loopback && !private(addr) {
        return Err(Error::Public(addr));
    }
    if !tls && !loopback {
        return Err(Error::TlsRequired(addr));
    }
    Ok(())
}

fn private(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_private() || in_vpn_range(v4),
        IpAddr::V6(v6) => unique_local(v6),
    }
}

/// `100.64.0.0/10`, the shared address space the VPN hands out (VPN-001).
fn in_vpn_range(addr: Ipv4Addr) -> bool {
    let [first, second, ..] = addr.octets();
    first == 100 && second & 0b1100_0000 == 0b0100_0000
}

/// `fc00::/7`.
fn unique_local(addr: Ipv6Addr) -> bool {
    addr.segments()[0] & 0xfe00 == 0xfc00
}
