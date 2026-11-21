//! The reverse proxy of the public names of applications (HOST-046, R-NEW-hubrouting; task H21;
//! Section 12.3 of `docs/proposals/hosted-applications-stage-2.md`).
//!
//! It is a process apart from the hub. It holds the certificates of the names it serves and
//! reads each request from the internet, so it gets only what [`wire`] gives: the table of the
//! names that serve and a connection to an application by its identifier
//! (SF-NEW-routeproxyexit). It cannot name a device, an address or a port.

pub mod door_header;
pub mod head;
pub mod hub;
pub mod limits;
pub mod plain;
pub mod relay;
pub mod serve;
pub mod settings;
pub mod tls;
pub mod wire;

#[cfg(test)]
mod tests;
