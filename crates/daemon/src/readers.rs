//! Reads this computer (MET-002): the `/proc` files and fixed commands, nothing written, and
//! the probes of the tool inventory (CAP-028). Parsing is `daemon_core::readings` and
//! `daemon_core::inventory`; this module only fetches the bytes (MET-016).

pub mod agent_sessions;
pub mod command;
pub mod gpu;
mod inventory;
mod linux;
mod macos;
#[cfg(windows)]
mod windows;

use chrono::{DateTime, Utc};
use daemon_core::readings::Reading;
pub use inventory::inventory;

/// What this computer is, which decides the commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Macos,
    #[cfg(windows)]
    Windows,
}

impl Platform {
    /// The platform the daemon was built for.
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(windows)]
        {
            Self::Windows
        }
        #[cfg(unix)]
        if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }
}

/// One collection of the CPU, memory, load, uptime, disks and interfaces. A reader that fails
/// leaves its field empty, so the sample is missing and the daemon carries on.
pub async fn reading(platform: Platform, now: DateTime<Utc>) -> Reading {
    match platform {
        Platform::Linux => linux::reading().await,
        Platform::Macos => macos::reading(now).await,
        #[cfg(windows)]
        Platform::Windows => windows::reading().await,
    }
}
