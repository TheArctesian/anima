//! The tool inventory of a device (CAP-020 to CAP-028): which developer tools it has, their
//! versions, its Rust build targets, its processor architecture and its operating system
//! version.
//!
//! Pure: the fixed probe list ([`PROBES`], [`TARGETS`]), the parse of each probe's output, the
//! folders a program is looked for in ([`candidates`]) and the inventory those make. The agent
//! runs the programs (CAP-028); the hub cannot send a probe or a command line (CAP-021).

mod probes;
mod search;
#[cfg(test)]
mod tests;
mod tools;
mod version;

pub use probes::{PROBES, Probe, Program, TARGETS, build_targets};
pub use search::{TOOL_FOLDERS, candidates, may_run};
pub use tools::{Reason, Tool, ToolName, Verdict};
pub use version::{
    MAX_TEXT, is_build_target, is_tool_name, is_version, kernel_release, macos_version,
    os_release_version,
};

use serde::{Deserialize, Serialize};

/// The most tools a batch may list (K8 Section 4).
pub const MAX_TOOLS: usize = 64;

/// The most build targets a batch may list (K8 Section 4).
pub const MAX_BUILD_TARGETS: usize = 128;

/// What a device's agent found installed (CAP-020, CAP-026).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub architecture: Architecture,
    pub os: Os,
    /// 1 to 64 bytes of [`is_version`] text, for example `27.0` or `24.04`.
    pub os_version: String,
    /// Each tool found once, in [`ToolName`] order. A tool not found is not listed.
    pub tools: Vec<Tool>,
    /// The installed Rust targets, unique, in byte order.
    pub build_targets: Vec<String>,
}

/// The processor architecture, as Rust names it (CAP-026).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    Aarch64,
    X86_64,
}

impl Architecture {
    /// The architecture of the name `std::env::consts::ARCH` gives; `None` for any other.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "aarch64" => Some(Self::Aarch64),
            "x86_64" => Some(Self::X86_64),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::X86_64 => "x86_64",
        }
    }
}

/// The operating system, as Rust names it (CAP-026).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Macos,
    Linux,
}

impl Os {
    /// The system of the name `std::env::consts::OS` gives; `None` for any other.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "macos" => Some(Self::Macos),
            "linux" => Some(Self::Linux),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Linux => "linux",
        }
    }
}

/// What became of one probe, as the agent's runner saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The program is in none of the folders.
    Missing,
    /// The program was found but ran past its time limit.
    TimedOut,
    /// The program was found but could not start, or exited with a failure.
    Exited,
    /// The program ran and exited with success; what it printed (at most the runner's cap).
    Printed(Vec<u8>),
}

impl Inventory {
    /// The inventory of a device of `architecture` and `os` at `os_version`, from the outcome of
    /// each probe of [`PROBES`] and the output of [`TARGETS`] (CAP-020). A probe whose program is
    /// missing leaves its tool out; one that failed or printed no version lists it as not usable,
    /// with the [`Reason`].
    #[must_use]
    pub fn new(
        (architecture, os, os_version): (Architecture, Os, String),
        outcomes: &[(&Probe, Outcome)],
        targets: Option<&[u8]>,
    ) -> Self {
        let mut tools: Vec<Tool> = outcomes
            .iter()
            .filter_map(|(probe, outcome)| probe.tool(outcome))
            .collect();
        tools.sort_by_key(|tool| tool.name);
        tools.dedup_by_key(|tool| tool.name);
        Self {
            architecture,
            os,
            os_version,
            tools,
            build_targets: targets.map_or_default(build_targets),
        }
    }
}
