//! What the session tunnels serve with (K3): built once from the hub's dependencies and lent
//! to each router.

use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    clock::Clock,
    fleet::{credentials::Vault, events::Bus, sessions::Live},
};

/// Everything the tunnels need, built once (replaces the long `tunnel_router` and
/// `terminal_router` argument lists).
#[derive(Clone)]
pub struct Serving {
    pub db: PgPool,
    pub vault: Vault,
    pub bus: Bus,
    pub clock: Arc<dyn Clock>,
    /// The tickets and tunnels, and the target rule for the device's host (R-032).
    pub live: Live,
    /// guacd's `host:port` (K7, R-032 E-8).
    pub guacd: String,
    /// `ANIMA_GUACD_DRIVE_ROOT`: where guacd keeps each device's shared drive (K7, RA-021).
    pub drive_root: String,
}
