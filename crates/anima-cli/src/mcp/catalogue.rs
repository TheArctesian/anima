//! Which tools the server offers: the token's kind, the offered sets and the sets that may
//! change things decide (MCP-002, MCP-003). Pure: the server and the tests ask the same
//! question.

use crate::hub::TokenKind;

/// A group of tools for one operator noun (`--toolsets`, MCP-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ToolSet {
    Devices,
    Checks,
    Maintenance,
    Events,
    Notifications,
    Web,
    Releases,
    Vpn,
    Retention,
}

/// What a tool needs from the token and the permissions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Reads only; always offered.
    Read,
    /// Changes something; offered when its set may change things. A read-scope key gets
    /// FORBIDDEN from the hub.
    Write,
    /// Changes something an API key may never do (INT-003, "L" in the plan's table); offered
    /// only for a login token, when its set may change things.
    Login,
}

/// What the operator permitted, with the kind of token the server holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub token: TokenKind,
    /// The sets to offer; empty offers every set.
    pub toolsets: Vec<ToolSet>,
    /// The sets whose change tools are offered; empty is read-only.
    pub write: Vec<ToolSet>,
}

impl Grant {
    /// Whether a tool of `set` that needs `access` is offered.
    #[must_use]
    pub fn offers(&self, set: ToolSet, access: Access) -> bool {
        let in_sets = self.toolsets.is_empty() || self.toolsets.contains(&set);
        let permitted = match access {
            Access::Read => true,
            Access::Write => self.write.contains(&set),
            Access::Login => self.write.contains(&set) && self.token == TokenKind::Login,
        };
        in_sets && permitted
    }

    /// The offered sets, in the catalogue's order.
    #[must_use]
    pub fn sets(&self) -> Vec<ToolSet> {
        ToolSet::ALL
            .into_iter()
            .filter(|set| self.toolsets.is_empty() || self.toolsets.contains(set))
            .collect()
    }

    /// The offered sets that may change things.
    #[must_use]
    pub fn writable(&self) -> Vec<ToolSet> {
        self.sets()
            .into_iter()
            .filter(|set| self.write.contains(set))
            .collect()
    }
}

impl ToolSet {
    pub const ALL: [Self; 9] = [
        Self::Devices,
        Self::Checks,
        Self::Maintenance,
        Self::Events,
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

/// `sets` by name, joined with commas; `none` when empty.
#[must_use]
pub fn names(sets: &[ToolSet]) -> String {
    if sets.is_empty() {
        return "none".to_owned();
    }
    sets.iter()
        .map(|set| set.name())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(token: TokenKind, toolsets: &[ToolSet], write: &[ToolSet]) -> Grant {
        Grant {
            token,
            toolsets: toolsets.to_vec(),
            write: write.to_vec(),
        }
    }

    #[test]
    fn mcp_002_with_no_write_set_only_read_tools_are_offered() {
        for token in [TokenKind::Login, TokenKind::ApiKey] {
            let read_only = grant(token, &[], &[]);
            assert!(read_only.offers(ToolSet::Devices, Access::Read));
            assert!(!read_only.offers(ToolSet::Devices, Access::Write));
            assert!(!read_only.offers(ToolSet::Devices, Access::Login));
            assert_eq!(read_only.writable(), Vec::<ToolSet>::new());
        }
    }

    #[test]
    fn mcp_003_an_api_key_never_gets_login_tools_even_where_writing_is_permitted() {
        let key = grant(TokenKind::ApiKey, &[], &ToolSet::ALL);
        assert!(key.offers(ToolSet::Checks, Access::Write));
        assert!(!key.offers(ToolSet::Retention, Access::Login));
        let login = grant(TokenKind::Login, &[], &ToolSet::ALL);
        assert!(login.offers(ToolSet::Retention, Access::Login));
    }

    #[test]
    fn mcp_002_write_is_permitted_per_set() {
        let checks = grant(TokenKind::Login, &[], &[ToolSet::Checks]);
        assert!(checks.offers(ToolSet::Checks, Access::Write));
        assert!(!checks.offers(ToolSet::Devices, Access::Write));
        assert!(!checks.offers(ToolSet::Devices, Access::Login));
        assert!(checks.offers(ToolSet::Devices, Access::Read));
        assert_eq!(checks.writable(), vec![ToolSet::Checks]);
    }

    #[test]
    fn mcp_002_toolsets_limit_the_sets_and_write_outside_them_counts_for_nothing() {
        let devices = grant(TokenKind::Login, &[ToolSet::Devices], &ToolSet::ALL);
        assert!(devices.offers(ToolSet::Devices, Access::Login));
        assert!(!devices.offers(ToolSet::Checks, Access::Read));
        assert!(!devices.offers(ToolSet::Checks, Access::Write));
        assert_eq!(devices.sets(), vec![ToolSet::Devices]);
        assert_eq!(devices.writable(), vec![ToolSet::Devices]);
        assert_eq!(grant(TokenKind::Login, &[], &[]).sets().len(), 9);
        assert_eq!(names(&devices.writable()), "devices");
        assert_eq!(ToolSet::named("vpn"), Some(ToolSet::Vpn));
    }
}
