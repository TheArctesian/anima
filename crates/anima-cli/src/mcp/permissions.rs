//! Which tool sets `anima mcp` offers and which of them may change things (MCP-002): saved
//! per hub in the configuration file, overridden for one run by the flags at start. With
//! nothing saved and no flag the server is read-only.

use serde::Serialize;

use super::catalogue::{ToolSet, names};
use crate::contexts::config::McpPermissions;
use crate::failure::Failure;

/// The operator's permissions, by set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Permissions {
    /// The sets offered; empty offers every set.
    #[serde(serialize_with = "serialise_sets")]
    pub toolsets: Vec<ToolSet>,
    /// The sets whose change tools are offered; empty is read-only.
    #[serde(serialize_with = "serialise_sets")]
    pub write: Vec<ToolSet>,
}

/// What the flags at start say; `None` keeps the saved value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Override {
    pub toolsets: Option<Vec<ToolSet>>,
    /// `Some(empty)` permits every set (`--allow-write` alone).
    pub write: Option<Vec<ToolSet>>,
    /// `--read-only`: no set may change things.
    pub read_only: bool,
}

impl Permissions {
    /// The saved permissions of one hub.
    ///
    /// # Errors
    /// [`Failure::Usage`] when the file names a set that does not exist (edited by hand).
    pub fn from_saved(saved: &McpPermissions) -> Result<Self, Failure> {
        Ok(Self {
            toolsets: parse_names(&saved.toolsets)?,
            write: parse_names(&saved.write)?,
        })
    }

    /// The form the configuration file keeps.
    #[must_use]
    pub fn to_saved(&self) -> McpPermissions {
        let named = |sets: &[ToolSet]| sets.iter().map(|set| set.name().to_owned()).collect();
        McpPermissions {
            toolsets: named(&self.toolsets),
            write: named(&self.write),
        }
    }

    /// These permissions with `flags` laid over them.
    #[must_use]
    pub fn overridden(mut self, flags: &Override) -> Self {
        if let Some(toolsets) = &flags.toolsets {
            self.toolsets.clone_from(toolsets);
        }
        if let Some(write) = &flags.write {
            self.write = every_set_when_empty(write);
        }
        if flags.read_only {
            self.write.clear();
        }
        self
    }

    /// The offered sets, in the catalogue's order.
    #[must_use]
    pub fn offered(&self) -> Vec<ToolSet> {
        ToolSet::ALL
            .into_iter()
            .filter(|set| self.toolsets.is_empty() || self.toolsets.contains(set))
            .collect()
    }

    /// The plain statement of what these permissions allow, for `permissions show` and the
    /// server's instructions.
    #[must_use]
    pub fn statement(&self) -> String {
        let offered = self.offered();
        let (writable, read_only): (Vec<ToolSet>, Vec<ToolSet>) =
            offered.iter().partition(|set| self.write.contains(set));
        format!(
            "Tool sets on: {}. May change things: {}. Read-only: {}. The hub token is still \
             the final limit: an API key with read scope cannot change anything, and the tools \
             that need a login token (device update and delete, release create, build cancel, \
             VPN route approval, retention set) are offered only with a login token.",
            names(&offered),
            names(&writable),
            names(&read_only),
        )
    }
}

/// `--allow-write` and `--write` alone (no names) permit every set.
#[must_use]
pub fn every_set_when_empty(sets: &[ToolSet]) -> Vec<ToolSet> {
    if sets.is_empty() {
        ToolSet::ALL.to_vec()
    } else {
        sets.to_vec()
    }
}

fn parse_names(names: &[String]) -> Result<Vec<ToolSet>, Failure> {
    names
        .iter()
        .map(|name| {
            ToolSet::named(name).ok_or_else(|| Failure::Usage {
                message: format!(
                    "The saved MCP permissions name the tool set {name}, which does not exist. \
                     Valid sets: {}.",
                    names_of_all()
                ),
                hint: Some("anima mcp permissions set --help".to_owned()),
            })
        })
        .collect()
}

fn names_of_all() -> String {
    names(&ToolSet::ALL)
}

fn serialise_sets<S: serde::Serializer>(
    sets: &[ToolSet],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(sets.iter().map(|set| set.name()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_002_nothing_saved_and_no_flag_is_read_only() {
        let permissions = Permissions::default().overridden(&Override::default());
        assert_eq!(permissions.write, Vec::<ToolSet>::new());
        assert_eq!(permissions.offered().len(), 9);
        assert!(permissions.statement().contains("May change things: none."));
    }

    #[test]
    fn mcp_002_a_flag_overrides_the_saved_value_for_the_run() {
        let saved = Permissions {
            toolsets: vec![ToolSet::Devices, ToolSet::Checks],
            write: vec![ToolSet::Checks],
        };
        let all = saved.clone().overridden(&Override {
            write: Some(Vec::new()),
            ..Override::default()
        });
        assert_eq!(all.write, ToolSet::ALL.to_vec());
        let read_only = saved.clone().overridden(&Override {
            read_only: true,
            ..Override::default()
        });
        assert_eq!(read_only.write, Vec::<ToolSet>::new());
        let other = saved.overridden(&Override {
            toolsets: Some(vec![ToolSet::Vpn]),
            ..Override::default()
        });
        assert_eq!(other.offered(), vec![ToolSet::Vpn]);
        assert_eq!(other.write, vec![ToolSet::Checks]);
    }

    #[test]
    fn mcp_002_saved_permissions_round_trip_and_unknown_names_are_refused() {
        let permissions = Permissions {
            toolsets: Vec::new(),
            write: vec![ToolSet::Checks, ToolSet::Maintenance],
        };
        let saved = permissions.to_saved();
        assert_eq!(saved.write, vec!["checks", "maintenance"]);
        assert_eq!(Permissions::from_saved(&saved).ok(), Some(permissions));
        let wrong = McpPermissions {
            toolsets: vec!["printers".to_owned()],
            write: Vec::new(),
        };
        let error = Permissions::from_saved(&wrong).expect_err("unknown set");
        assert!(error.to_string().contains("devices, checks"), "{error}");
    }

    #[test]
    fn mcp_002_the_statement_names_on_writable_and_read_only_sets() {
        let permissions = Permissions {
            toolsets: vec![ToolSet::Devices, ToolSet::Checks],
            write: vec![ToolSet::Checks],
        };
        let text = permissions.statement();
        assert!(text.contains("Tool sets on: devices, checks."), "{text}");
        assert!(text.contains("May change things: checks."), "{text}");
        assert!(text.contains("Read-only: devices."), "{text}");
        assert!(text.contains("read scope"), "{text}");
    }
}
