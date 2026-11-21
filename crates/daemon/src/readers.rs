//! Reads this computer (MET-002): the `/proc` files and fixed commands, nothing written, and
//! the probes of the tool inventory (CAP-028). Parsing is `daemon_core::readings` and
//! `daemon_core::inventory`; this module only fetches the bytes (MET-016).

pub mod agent_sessions;
pub mod command;
pub mod container;
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

/// What the daemon reads: the machine it runs on, or the container of a hosted hub, where a
/// reading of the machine would show the server (HMD-071, HMD-074).
#[derive(Debug)]
pub enum Source {
    Machine(Platform),
    HostedHub(container::Container),
}

impl Source {
    /// One collection at `now` (the wall clock for macOS, the monotonic one for the container).
    pub async fn reading(&mut self, now: DateTime<Utc>, instant: std::time::Instant) -> Reading {
        match self {
            Self::Machine(platform) => reading(*platform, now).await,
            Self::HostedHub(container) => container.reading(instant).await,
        }
    }

    /// Whether GPUs are looked for: never in a container, where a GPU would be one of the server.
    #[must_use]
    pub const fn has_gpus(&self) -> bool {
        matches!(self, Self::Machine(_))
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
