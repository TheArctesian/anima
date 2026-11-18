//! The tools the hub knows by name (CAP-023), and what the daemon found of each (CAP-024). A
//! name a newer daemon sends that is not here is ignored by the hub, so the two can differ in
//! version (K8 Section 4).

use serde::{Deserialize, Serialize};

/// A tool of the inventory. The order is the order of the list the daemon sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolName {
    Git,
    Rustc,
    Cargo,
    CargoNextest,
    Sccache,
    Bun,
    Node,
    Xcodebuild,
    Docker,
    Podman,
    Nix,
    Claude,
    Codex,
}

impl ToolName {
    pub const ALL: [Self; 13] = [
        Self::Git,
        Self::Rustc,
        Self::Cargo,
        Self::CargoNextest,
        Self::Sccache,
        Self::Bun,
        Self::Node,
        Self::Xcodebuild,
        Self::Docker,
        Self::Podman,
        Self::Nix,
        Self::Claude,
        Self::Codex,
    ];

    /// The name a batch and the API spell, which is also the program's name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Rustc => "rustc",
            Self::Cargo => "cargo",
            Self::CargoNextest => "cargo-nextest",
            Self::Sccache => "sccache",
            Self::Bun => "bun",
            Self::Node => "node",
            Self::Xcodebuild => "xcodebuild",
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Nix => "nix",
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// The tool named `text`; `None` for a name this version does not know.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.as_str() == text)
    }
}

/// One tool the daemon found (CAP-024).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tool {
    pub name: ToolName,
    pub verdict: Verdict,
}

/// Whether a found tool can be used: its version, or why not (CAP-024, CAP-002).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The probe printed this version.
    Usable(String),
    NotUsable(Reason),
}

/// Why a found tool is not usable: a fixed code, never text from the device. K8 spells it as
/// the `state` of a tool; a state this version does not know reads as [`Reason::Failed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The version probe ran past its time limit.
    TimedOut,
    /// The program could not start, or exited with a failure.
    Exited,
    /// The program ran but printed no version the parser accepts.
    NoVersion,
    /// A reason a newer daemon knows and this version does not.
    #[serde(other)]
    Failed,
}

impl Reason {
    /// The code K8 and the database spell.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TimedOut => "timed_out",
            Self::Exited => "exited",
            Self::NoVersion => "no_version",
            Self::Failed => "failed",
        }
    }

    /// The reason spelled `text`; [`Reason::Failed`] for a code this version does not know.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        [Self::TimedOut, Self::Exited, Self::NoVersion]
            .into_iter()
            .find(|reason| reason.as_str() == text)
            .unwrap_or(Self::Failed)
    }
}

impl Tool {
    /// The version, when the tool is usable.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        match &self.verdict {
            Verdict::Usable(version) => Some(version),
            Verdict::NotUsable(_) => None,
        }
    }

    /// Why the tool is not usable; `None` when it is.
    #[must_use]
    pub const fn reason(&self) -> Option<Reason> {
        match self.verdict {
            Verdict::Usable(_) => None,
            Verdict::NotUsable(reason) => Some(reason),
        }
    }

    /// Whether the tool can be used: its probe gave a version (CAP-024).
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        matches!(self.verdict, Verdict::Usable(_))
    }
}
