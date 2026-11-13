//! One change to a hub's saved MCP permissions, and the one path that reads and saves them
//! (MCP-014, MCP-018). `anima mcp permissions` and the desktop Settings panel both go through
//! here, so the file format and its atomic replace live in one place. Pure apart from `read`,
//! `edit` and `save`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::catalogue::{Kind, ToolSet};
use super::permissions::{Permissions, Scope, State, resolve};
use super::selection::{Flags, Row, select, statement};
use super::tools::{Entry, catalogue};
use crate::contexts::config::Config;
use crate::failure::Failure;
use crate::hub::Origin;

/// What the operator asks for. The panel sends it as `{ "type": "choose" | "bulk" | "reset", ... }`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Change {
    /// `state` for these tools or tool sets, by name.
    Choose { names: Vec<String>, state: State },
    /// `state` for every tool in scope: one set or all, one kind or both.
    Bulk {
        #[serde(default)]
        toolset: Option<ToolSet>,
        #[serde(default)]
        kind: Option<Kind>,
        state: State,
    },
    /// Forget the saved states: read tools allow, change tools off.
    Reset,
}

impl Change {
    /// Whether this turns change tools on without a question per call; the operator confirms
    /// it first (MCP-014, MCP-020).
    #[must_use]
    pub fn allows_changes(&self) -> bool {
        matches!(
            self,
            Self::Bulk { state: State::Allow, kind, .. } if *kind != Some(Kind::Read)
        )
    }

    /// Lays the change over `permissions`.
    ///
    /// # Errors
    /// [`Failure::Usage`] naming a tool or set that does not exist.
    pub fn apply(&self, permissions: &mut Permissions, catalogue: &[Entry]) -> Result<(), Failure> {
        match self {
            Self::Choose { names, state } => {
                for tool in resolve(names, catalogue)? {
                    permissions.choose(tool, *state);
                }
            }
            Self::Bulk {
                toolset,
                kind,
                state,
            } => {
                let scope = Scope {
                    set: *toolset,
                    kind: *kind,
                };
                permissions.choose_all(*state, scope, catalogue);
            }
            Self::Reset => *permissions = Permissions::default(),
        }
        Ok(())
    }
}

/// One row of `permissions show --json`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownTool {
    pub toolset: &'static str,
    pub tool: &'static str,
    pub kind: Kind,
    pub needs_sign_in: bool,
    pub state: State,
    pub description: &'static str,
}

/// `permissions show --json`.
#[derive(Debug, Serialize)]
pub struct Shown {
    pub hub: String,
    pub summary: String,
    pub tools: Vec<ShownTool>,
}

/// Every tool with its saved state for `origin`; with nothing given, the built-in defaults.
#[must_use]
pub fn shown(origin: &Origin, permissions: &Permissions) -> Shown {
    let rows = select(catalogue(), None, permissions, &Flags::default());
    Shown {
        hub: origin.as_text(),
        summary: statement(&rows),
        tools: rows.iter().map(shown_tool).collect(),
    }
}

fn shown_tool(row: &Row) -> ShownTool {
    let definition = &row.entry.definition;
    ShownTool {
        toolset: row.entry.set.name(),
        tool: row.entry.name,
        kind: row.entry.kind(),
        needs_sign_in: definition.access.needs_sign_in(),
        state: row.state,
        description: definition.description,
    }
}

/// The saved permissions of `origin` in `config`; nothing saved is the built-in defaults.
///
/// # Errors
/// [`Failure`] for a saved name that does not exist.
pub fn saved_in(config: &Config, origin: &Origin) -> Result<Permissions, Failure> {
    match config.mcp.get(&origin.as_text()) {
        Some(saved) => Permissions::from_saved(saved, &catalogue()),
        None => Ok(Permissions::default()),
    }
}

/// Writes `permissions` for `origin` into the configuration in `dir`, whole, by rename. The
/// built-in defaults are saved as no entry.
///
/// # Errors
/// [`Failure::Config`] when the file cannot be written.
pub fn save(
    mut config: Config,
    dir: &Path,
    origin: &Origin,
    permissions: &Permissions,
) -> Result<(), Failure> {
    let key = origin.as_text();
    if *permissions == Permissions::default() {
        config.mcp.remove(&key);
    } else {
        config.mcp.insert(key, permissions.to_saved());
    }
    Ok(config.save(dir)?)
}

/// The saved permissions of `origin`, shown.
///
/// # Errors
/// [`Failure`] for an unreadable configuration or a saved name that does not exist.
pub fn read(dir: &Path, origin: &Origin) -> Result<Shown, Failure> {
    let config = Config::load(dir)?;
    Ok(shown(origin, &saved_in(&config, origin)?))
}

/// Applies `change` to the saved permissions of `origin` and saves them.
///
/// # Errors
/// [`Failure`] for an unreadable or unwritable configuration, or a name that does not exist.
pub fn edit(dir: &Path, origin: &Origin, change: &Change) -> Result<Shown, Failure> {
    let config = Config::load(dir)?;
    let mut permissions = saved_in(&config, origin)?;
    change.apply(&mut permissions, &catalogue())?;
    save(config, dir, origin, &permissions)?;
    Ok(shown(origin, &permissions))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub() -> Origin {
        Origin::parse("https://hub.example.com").expect("an origin")
    }

    fn state_of(shown: &Shown, tool: &str) -> State {
        shown
            .tools
            .iter()
            .find(|row| row.tool == tool)
            .map_or(State::Off, |row| row.state)
    }

    fn change(json: &str) -> Change {
        serde_json::from_str(json).expect("a change")
    }

    #[test]
    fn mcp_018_the_panel_sends_each_change_as_tagged_json() {
        let one = change(r#"{"type":"choose","names":["anima_check_update"],"state":"ask"}"#);
        assert_eq!(
            one,
            Change::Choose {
                names: vec!["anima_check_update".to_owned()],
                state: State::Ask
            }
        );
        let set = change(r#"{"type":"bulk","toolset":"vpn","kind":"change","state":"off"}"#);
        assert_eq!(
            set,
            Change::Bulk {
                toolset: Some(ToolSet::Vpn),
                kind: Some(Kind::Change),
                state: State::Off
            }
        );
        let all = change(r#"{"type":"bulk","state":"allow"}"#);
        assert!(matches!(
            all,
            Change::Bulk {
                toolset: None,
                kind: None,
                ..
            }
        ));
        assert_eq!(change(r#"{"type":"reset"}"#), Change::Reset);
    }

    #[test]
    fn mcp_020_only_a_bulk_allow_that_covers_change_tools_needs_confirmation() {
        assert!(change(r#"{"type":"bulk","state":"allow"}"#).allows_changes());
        assert!(change(r#"{"type":"bulk","toolset":"web","state":"allow"}"#).allows_changes());
        assert!(!change(r#"{"type":"bulk","kind":"read","state":"allow"}"#).allows_changes());
        assert!(!change(r#"{"type":"bulk","state":"ask"}"#).allows_changes());
        assert!(!change(r#"{"type":"reset"}"#).allows_changes());
    }

    #[test]
    fn mcp_018_an_edit_is_saved_to_the_file_and_read_back() {
        let dir = tempfile::tempdir().expect("a folder");
        let before = read(dir.path(), &hub()).expect("read");
        assert_eq!(state_of(&before, "anima_check_update"), State::Off);
        let shown = edit(
            dir.path(),
            &hub(),
            &change(r#"{"type":"bulk","toolset":"checks","state":"ask"}"#),
        )
        .expect("edit");
        assert_eq!(state_of(&shown, "anima_check_update"), State::Ask);
        let again = read(dir.path(), &hub()).expect("read");
        assert_eq!(state_of(&again, "anima_check_update"), State::Ask);
        assert_eq!(state_of(&again, "anima_device_create"), State::Off);
    }

    #[test]
    fn mcp_018_reset_removes_the_hubs_entry_and_leaves_other_hubs_alone() {
        let dir = tempfile::tempdir().expect("a folder");
        let other = Origin::parse("https://other.example.com").expect("an origin");
        let allow = change(r#"{"type":"bulk","kind":"change","state":"allow"}"#);
        edit(dir.path(), &hub(), &allow).expect("edit");
        edit(dir.path(), &other, &allow).expect("edit");
        let shown = edit(dir.path(), &hub(), &Change::Reset).expect("reset");
        assert_eq!(state_of(&shown, "anima_check_update"), State::Off);
        let config = Config::load(dir.path()).expect("config");
        assert_eq!(config.mcp.len(), 1);
        assert!(config.mcp.contains_key(&other.as_text()));
    }

    #[test]
    fn mcp_018_an_unknown_tool_changes_nothing() {
        let dir = tempfile::tempdir().expect("a folder");
        let bad = change(r#"{"type":"choose","names":["anima_nothing"],"state":"allow"}"#);
        assert!(matches!(
            edit(dir.path(), &hub(), &bad),
            Err(Failure::Usage { .. })
        ));
        assert!(!dir.path().join("config.json").exists());
    }
}
