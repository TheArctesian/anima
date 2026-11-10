//! Tests of the domain expiry check (spec 6.4, CHK-009, D-29, P3-003). Servers listen on
//! loopback only; no test queries a real RDAP or IANA server.

mod bootstrap;
mod config;
mod egress;
mod observe;
pub(in crate::checks) mod registry;
mod reply;
pub(in crate::checks) mod server;
mod status;
