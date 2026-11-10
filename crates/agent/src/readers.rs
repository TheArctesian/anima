//! Reads this computer (MET-002): the `/proc` files and fixed commands, nothing written.
//! Parsing is `agent_core::readings`; this module only fetches the bytes (MET-016).

pub mod command;
pub mod gpu;
mod linux;
mod macos;

use agent_core::readings::Reading;
use chrono::{DateTime, Utc};

/// What this computer is, which decides the commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Macos,
}

impl Platform {
    /// The platform the agent was built for.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }
}

/// One collection of the CPU, memory, load, uptime, disks and interfaces. A reader that fails
/// leaves its field empty, so the sample is missing and the agent carries on.
pub async fn reading(platform: Platform, now: DateTime<Utc>) -> Reading {
    match platform {
        Platform::Linux => linux::reading().await,
        Platform::Macos => macos::reading(now).await,
    }
}
