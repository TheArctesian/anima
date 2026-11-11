//! The VPN picture through the hub: the reader's log lines, the `vpn` query and the
//! `vpnChanged` subscription (VPN-005, VPN-008 to VPN-011).

mod broker;
mod cancelled_audit;
mod graphql;
mod key_refusal;
mod mutations;
mod proxy;
mod tailnet_site;
