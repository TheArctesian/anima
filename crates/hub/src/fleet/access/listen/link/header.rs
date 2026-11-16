//! The header `anima-link` puts at the start of each connection to the hub's socket: a PROXY
//! protocol header, version 2, whose source address is the link address of the device whose
//! key the link authenticated (`link/hublink/deviceheader.go` writes it). Pure.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use crate::config::LINK_DEVICES;

/// The header's length: the link always sends two IPv6 addresses, two ports and nothing more.
pub(super) const LENGTH: usize = 52;

/// The protocol's 12 fixed bytes, then version 2 with command PROXY, then TCP over IPv6, then
/// the length of what follows: 36.
const START: [u8; 16] = [
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A, 0x21, 0x21, 0x00, 0x24,
];

/// The device `header` names, as the peer of its connection: its link address and the port
/// its stream came from. `None` when the bytes are not the link's header or the address is
/// not a link address, so no other address can be named through the socket.
pub(super) fn device(header: &[u8; LENGTH]) -> Option<SocketAddr> {
    let (start, addresses) = header.split_at(START.len());
    if start != START {
        return None;
    }
    let source: [u8; 16] = addresses.get(..16)?.try_into().ok()?;
    let source = Ipv6Addr::from(source);
    if !LINK_DEVICES.contains(&source) {
        return None;
    }
    let port = u16::from_be_bytes([*addresses.get(32)?, *addresses.get(33)?]);
    Some(SocketAddr::new(IpAddr::V6(source), port))
}
