use std::{
    net::{Ipv4Addr, SocketAddr, TcpStream},
    time::Duration,
};

/// Whether `ip` is inside 100.64.0.0/10, the range the Anima VPN gives its nodes (NET-001).
/// Other overlays (Tailscale, `NetBird`) use the same range, so this only checks an address the
/// Anima VPN client gave; it never says which network an address is on (ADR-005).
pub(super) fn is_vpn_address(ip: Ipv4Addr) -> bool {
    let [first, second, ..] = ip.octets();
    first == 100 && second & 0b1100_0000 == 0b0100_0000
}

/// Whether a TCP connection to `address` is accepted within `patience`. The connection is
/// closed at once; nothing is sent.
pub(super) fn accepts_connection(address: SocketAddr, patience: Duration) -> bool {
    TcpStream::connect_timeout(&address, patience).is_ok()
}

/// Whether SSH answers on this computer's VPN address: a TCP connection to `port` there is accepted within
/// `patience`. False without a VPN address.
pub(super) fn ssh_answers(vpn_address: Option<Ipv4Addr>, port: u16, patience: Duration) -> bool {
    vpn_address.is_some_and(|ip| accepts_connection((ip, port).into(), patience))
}
