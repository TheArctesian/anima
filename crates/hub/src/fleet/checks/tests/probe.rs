//! Tests for check configuration and probes. Servers listen on 127.0.0.1 only; nothing
//! reaches the internet.

mod config;
mod dns;
mod downgrade;
mod http;
mod icmp;
mod listing;
mod messages;
mod proxy;
mod reach;
mod secrets;
pub(super) mod support;
mod tcp;
mod tls;
