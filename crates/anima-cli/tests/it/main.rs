//! `anima` run as a program against a scripted hub on a loopback port. The tests keep tokens
//! in the 0600 token file (`ANIMA_TOKEN_STORE=file`), never in this computer's key store.
//!
//! One module for each noun (`devices`); `basics` holds the commands every noun relies on.

mod agents;
mod basics;
mod builders;
mod checks;
mod coverage;
mod device_changes;
mod device_secrets;
mod device_trust;
mod devices;
mod events;
mod keys;
mod kit;
mod maintenance;
mod mcp;
mod notifications;
mod releases;
mod retention;
mod sites;
mod vpn;
mod web_interfaces;
