//! A loopback hub origin as the client saves it (F-1): `http://127.0.0.1:<port>`,
//! `http://localhost:<port>` or `http://[::1]:<port>`, nothing more. Pure.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The loopback addresses an origin's host can reach, and its port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackOrigin {
    pub addresses: Vec<IpAddr>,
    pub port: u16,
}

/// The origin, or none when it is not one of the three loopback forms with a port.
pub fn parse(origin: &str) -> Option<LoopbackOrigin> {
    let v4 = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let v6 = IpAddr::V6(Ipv6Addr::LOCALHOST);
    let rest = origin.strip_prefix("http://")?;
    let (addresses, port) = if let Some(port) = rest.strip_prefix("127.0.0.1:") {
        (vec![v4], port)
    } else if let Some(port) = rest.strip_prefix("[::1]:") {
        (vec![v6], port)
    } else {
        // `localhost` can reach either address; both must be checked.
        (vec![v4, v6], rest.strip_prefix("localhost:")?)
    };
    let digits =
        !port.is_empty() && port.len() <= 5 && port.bytes().all(|byte| byte.is_ascii_digit());
    let port: u16 = port.parse().ok().filter(|_| digits)?;
    (port != 0).then_some(LoopbackOrigin { addresses, port })
}
