//! The fleet nouns: devices and how they are reached.

mod agents;
mod checks;
mod device_changes;
mod device_paging;
mod device_secrets;
mod device_trust;
pub mod devices;
mod link;

pub use link::device as link_device;
mod maintenance;
mod sessions;
mod sites;
mod watches;
mod web_interfaces;
