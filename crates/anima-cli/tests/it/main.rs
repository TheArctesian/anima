//! `anima` run as a program against a scripted hub on a loopback port. The tests keep tokens
//! in the 0600 token file (`ANIMA_TOKEN_STORE=file`), never in this computer's key store.
//!
//! One module for each noun (`devices`); `basics` holds the commands every noun relies on.

mod agents;
mod basics;
mod coverage;
mod device_changes;
mod device_secrets;
mod devices;
mod kit;
mod sites;
mod web_interfaces;
