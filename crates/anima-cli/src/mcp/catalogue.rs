//! The words of the tool catalogue: the tool sets, what a tool needs from the token, and its
//! kind (MCP-002, MCP-003).

use serde::{Deserialize, Serialize};

/// A group of tools for one operator noun (`--toolsets`, MCP-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ToolSet {
    Devices,
    Checks,
    Maintenance,
    Events,
    Sessions,
    Notifications,
    Web,
    Releases,
    Vpn,
    Retention,
}

/// What a tool needs from the token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Reads only.
    Read,
    /// Changes something; a read-scope key gets FORBIDDEN from the hub.
    Write,
    /// Changes something an API key may never do (INT-003, "L" in the plan's table); listed
    /// only for a login token (MCP-003).
    Login,
}

/// Whether a tool reads or changes things; each kind has its own default state (MCP-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Read,
    Change,
}

impl Access {
    #[must_use]
    pub const fn kind(self) -> Kind {
        match self {
            Self::Read => Kind::Read,
            Self::Write | Self::Login => Kind::Change,
        }
    }

    /// Whether only a login token may call the tool (INT-003).
    #[must_use]
    pub const fn needs_sign_in(self) -> bool {
        matches!(self, Self::Login)
    }
}

impl Kind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Change => "change",
        }
    }
}

impl ToolSet {
    pub const ALL: [Self; 10] = [
        Self::Devices,
        Self::Checks,
        Self::Maintenance,
        Self::Events,
        Self::Sessions,
        Self::Notifications,
        Self::Web,
        Self::Releases,
        Self::Vpn,
        Self::Retention,
    ];

    /// The set's name as `--toolsets` takes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Devices => "devices",
            Self::Checks => "checks",
            Self::Maintenance => "maintenance",
            Self::Events => "events",
            Self::Sessions => "sessions",
            Self::Notifications => "notifications",
            Self::Web => "web",
            Self::Releases => "releases",
            Self::Vpn => "vpn",
            Self::Retention => "retention",
        }
    }

    /// The set named `name`, as [`ToolSet::name`] writes it.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|set| set.name() == name)
    }
}

/// `names` joined with commas; `none` when empty.
#[must_use]
pub fn joined<S: AsRef<str>>(names: &[S]) -> String {
    if names.is_empty() {
        return "none".to_owned();
    }
    names
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(", ")
}

/// `sets` by name, joined with commas; `none` when empty.
#[must_use]
pub fn names(sets: &[ToolSet]) -> String {
    joined(&sets.iter().map(|set| set.name()).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_012_login_tools_are_change_tools_that_need_sign_in() {
        assert_eq!(Access::Read.kind(), Kind::Read);
        assert_eq!(Access::Write.kind(), Kind::Change);
        assert_eq!(Access::Login.kind(), Kind::Change);
        assert!(Access::Login.needs_sign_in());
        assert!(!Access::Write.needs_sign_in());
    }

    #[test]
    fn mcp_002_sets_have_names_both_ways() {
        for set in ToolSet::ALL {
            assert_eq!(ToolSet::named(set.name()), Some(set));
        }
        assert_eq!(names(&[ToolSet::Devices, ToolSet::Vpn]), "devices, vpn");
        assert_eq!(names(&[]), "none");
    }
}
