//! The tunnel between the operator's client and guacd (K4, GW-010 to GW-015).
//!
//! The rules are pure and live here: the `connect` values and time zone (`arguments`,
//! `known_hosts`), what each client and guacd instruction means after `ready` (`relay`,
//! `client`, `gateway`) and how a tunnel ends (`ending`). The tunnel task that does the I/O
//! comes with WP-10 T5.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "WP-10 T5 runs the tunnel with these rules")
)]

mod arguments;
mod client;
mod ending;
mod gateway;
mod known_hosts;
mod relay;
#[cfg(test)]
mod tests;
