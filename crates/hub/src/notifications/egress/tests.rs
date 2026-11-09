//! Proof of the one egress path (P3-003, D-17, N-2): every connection goes only to a checked
//! address of the configured host, a refused address gets no connection at all, no redirect
//! is followed, TLS is verified for the host name, and e-mail is encrypted before any login.
//! Local servers stand in for the targets; the policy allows loopback for them.

mod http;
pub(in crate::notifications) mod servers;
mod smtp;

use ipnet::IpNet;

use super::Egress;
use crate::targets::AddressPolicy;

/// An egress that may reach the local test servers.
pub(in crate::notifications) fn local() -> Egress {
    Egress::new(AddressPolicy::default().allowing_loopback())
}

/// An egress whose deny list covers every loopback address, though the policy allows loopback.
fn loopback_denied() -> Egress {
    let deny: Vec<IpNet> = vec!["127.0.0.0/8".parse().unwrap(), "::1/128".parse().unwrap()];
    Egress::new(AddressPolicy::new(deny).allowing_loopback())
}
