//! What the session tunnels serve with (K3): built once from the hub's dependencies and lent
//! to each router.

use std::{path::PathBuf, sync::Arc};

use sqlx::PgPool;

use crate::{clock::Clock, credentials::Vault, events::Bus, sessions::Live};

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
    /// Where and how much the hub records (K4 §11, WP-48).
    pub recording: Recording,
}

/// Recording storage (K4 §11): `ANIMA_RECORDING_ROOT` and `ANIMA_RECORDING_MAX_BYTES` (K7).
#[derive(Clone, Debug)]
pub struct Recording {
    /// The folder for recording files; `None` makes recording unavailable.
    pub root: Option<PathBuf>,
    /// The plaintext limit of one recording.
    pub max_bytes: u64,
}
