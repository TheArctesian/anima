//! The device manager: the domains that watch and reach the operator's devices, and the
//! operator access, credentials and events they share (REPO-E-001, OD-E5). Each domain keeps
//! the shape of Section 13 of the spec; K3 lists what one may use from another.

pub mod access;
pub mod builds;
pub mod checks;
pub mod credentials;
pub mod devices;
pub mod events;
pub mod maintenance;
pub mod metrics;
pub mod notifications;
pub mod retention;
pub mod sessions;
pub mod ssh;
pub mod targets;
pub mod trust;
pub mod vpn;
pub mod web_interfaces;
