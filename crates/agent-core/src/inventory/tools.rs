//! The tools the hub knows by name (CAP-023). A name a newer agent sends that is not here is
//! ignored by the hub, so the two can differ in version (K8 Section 4).

/// A tool of the inventory. The order is the order of the list the agent sends.
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
