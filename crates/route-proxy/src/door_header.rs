//! The PROXY protocol header (version 1) that the door of the cloud service puts before each
//! connection, with the address of the client (Section 12.2). The proxy reads it only when its
//! setting says that the connections come from the door; on a hub that a person hosts it never
//! reads it, so a client cannot claim an address (SF-NEW-routeheaders). Pure.

use std::net::IpAddr;

/// The longest version 1 header, with its line end (the specification gives 107).
pub const HEADER_MAX: usize = 107;

/// The client address of a version 1 header line, without its `\r\n`.
#[must_use]
pub fn client(line: &str) -> Option<IpAddr> {
    let mut parts = line.split(' ');
    let (Some("PROXY"), Some(family), Some(source)) = (parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let address: IpAddr = source.parse().ok()?;
    match (family, address) {
        ("TCP4", IpAddr::V4(_)) | ("TCP6", IpAddr::V6(_)) => Some(address),
        _ => None,
    }
}
