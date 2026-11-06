//! The tunnel between the operator's client and guacd (K4, GW-010 to GW-015, HUB-002).
//! Not mounted until RDP or VNC uses it: SSH uses the terminal tunnel (K4 Section 8, R-062).
//!
//! Pure rules: the `connect` values and time zone (`arguments`, `known_hosts`), what each
//! client and guacd instruction means after `ready` (`relay`, `client`, `gateway`) and how a
//! tunnel ends (`ending`). I/O: the `/tunnel` route and its checks before the upgrade
//! (`route`), the device, host-key and guacd side up to `ready` (`opening`, `guacd`), the
//! session task that owns the ticket from redeem to end (`relaying`); the end's row, audit entry and
//! event are written by `end::record`.

mod arguments;
mod client;
mod decimal;
mod ending;
mod gateway;
mod guacd;
mod known_hosts;
mod opening;
mod relay;
mod relaying;
mod route;
#[cfg(test)]
mod tests;

pub use route::tunnel_router;
