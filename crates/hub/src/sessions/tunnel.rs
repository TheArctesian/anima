//! The tunnel between the operator's client and guacd for RDP and VNC sessions (K4, GW-010
//! to GW-015, HUB-002). SSH uses the terminal tunnel (K4 Section 8, R-062).
//!
//! Pure rules: the protocol, `connect` values, audio and time zone (`arguments`), what each
//! client and guacd instruction means after `ready` (`relay`, `client`, `gateway`) and how a
//! tunnel ends (`ending`). I/O: the `/tunnel` route and its checks before the upgrade
//! (`route`), the device, certificate and guacd side up to `ready` (`opening`, `guacd`), the
//! session task that owns the ticket from redeem to end (`relaying`); the end's row, audit entry and
//! event are written by `end::record`.

mod arguments;
mod client;
mod decimal;
mod ending;
mod gateway;
mod guacd;
mod opening;
mod policy;
mod relay;
mod relaying;
mod route;
#[cfg(test)]
mod tests;

pub use route::tunnel_router;
