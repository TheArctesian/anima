//! The operator's saved choice for each MCP tool (MCP-002, MCP-012): `allow`, `ask` or `off`,
//! per tool, with a default for each kind. The saved form keeps only explicit choices and the
//! defaults, so a tool added later takes its kind's default. Pure: the commands read and write
//! the configuration file around it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::catalogue::{Kind, ToolSet, joined, names};
use super::tools::Entry;
use crate::contexts::config::McpPermissions;
use crate::failure::{Failure, Hint};

/// What the server does with one tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Listed; the agent may call it.
    Allow,
    /// Listed; each call needs the person's approval (MCP-013).
    Ask,
    /// Not listed, and refused if called.
    Off,
}

impl State {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Ask => "ask",
            Self::Off => "off",
        }
    }

    fn named(name: &str) -> Option<Self> {
        [Self::Allow, Self::Ask, Self::Off]
            .into_iter()
            .find(|state| state.name() == name)
    }

    /// The state of a kind with nothing saved: read tools `allow`, change tools `off`.
    #[must_use]
    pub const fn built_in(kind: Kind) -> Self {
        match kind {
            Kind::Read => Self::Allow,
            Kind::Change => Self::Off,
        }
    }
}

/// One hub's saved choices.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Permissions {
    /// The default of the read tools; `None` is the built-in `allow`.
    pub read: Option<State>,
    /// The default of the change tools; `None` is the built-in `off`.
    pub change: Option<State>,
    /// Explicit choices by tool name. A name no tool has (a tool removed later) is kept and
    /// does nothing.
    pub tools: BTreeMap<String, State>,
}

impl Permissions {
    /// The saved permissions of one hub; the form before MCP-012 (`toolsets`, `write`) becomes
    /// explicit choices: tools outside `toolsets` off, change tools of `write` allowed.
    ///
    /// # Errors
    /// [`Failure::Usage`] when the file names a state or a set that does not exist.
    pub fn from_saved(saved: &McpPermissions, catalogue: &[Entry]) -> Result<Self, Failure> {
        let mut permissions = Self {
            read: saved.read.as_deref().map(parse_state).transpose()?,
            change: saved.change.as_deref().map(parse_state).transpose()?,
            tools: BTreeMap::new(),
        };
        let offered = parse_sets(&saved.toolsets)?;
        let write = parse_sets(&saved.write)?;
        for entry in catalogue {
            if !offered.is_empty() && !offered.contains(&entry.set) {
                permissions.choose(entry.name, State::Off);
            } else if entry.kind() == Kind::Change && write.contains(&entry.set) {
                permissions.choose(entry.name, State::Allow);
            }
        }
        for (name, state) in &saved.tools {
            permissions.choose(name, parse_state(state)?);
        }
        Ok(permissions)
    }

    /// The form the configuration file keeps.
    #[must_use]
    pub fn to_saved(&self) -> McpPermissions {
        McpPermissions {
            read: self.read.map(|state| state.name().to_owned()),
            change: self.change.map(|state| state.name().to_owned()),
            tools: self
                .tools
                .iter()
                .map(|(name, state)| (name.clone(), state.name().to_owned()))
                .collect(),
            toolsets: Vec::new(),
            write: Vec::new(),
        }
    }

    /// The operator's saved choice for a tool: its own, else its kind's default; `None` when
    /// nothing is saved for it.
    #[must_use]
    pub fn choice(&self, name: &str, kind: Kind) -> Option<State> {
        self.tools.get(name).copied().or(match kind {
            Kind::Read => self.read,
            Kind::Change => self.change,
        })
    }

    /// The saved state of a tool, with the built-in default when nothing is saved.
    #[must_use]
    pub fn state(&self, name: &str, kind: Kind) -> State {
        self.choice(name, kind)
            .unwrap_or_else(|| State::built_in(kind))
    }

    /// Saves `state` for one tool.
    pub fn choose(&mut self, name: &str, state: State) {
        self.tools.insert(name.to_owned(), state);
    }

    /// Saves `state` for every tool in scope (`--allow-all` and the like). Without a set the
    /// kinds' defaults change and their explicit choices go, so tools added later follow;
    /// with a set each of its tools gets an explicit choice.
    pub fn choose_all(&mut self, state: State, scope: Scope, catalogue: &[Entry]) {
        let kinds: Vec<Kind> = scope
            .kind
            .map_or(vec![Kind::Read, Kind::Change], |kind| vec![kind]);
        let in_scope = catalogue
            .iter()
            .filter(|entry| kinds.contains(&entry.kind()))
            .filter(|entry| scope.set.is_none_or(|set| entry.set == set));
        if scope.set.is_some() {
            for entry in in_scope {
                self.choose(entry.name, state);
            }
            return;
        }
        for entry in in_scope {
            self.tools.remove(entry.name);
        }
        for kind in kinds {
            match kind {
                Kind::Read => self.read = Some(state),
                Kind::Change => self.change = Some(state),
            }
        }
    }
}

/// What a bulk choice covers: one set or all, one kind or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub set: Option<ToolSet>,
    pub kind: Option<Kind>,
}

/// The tools a list of names stands for: each name is a tool or a whole set.
///
/// # Errors
/// [`Failure::Usage`] (exit 2) naming the unknown name and listing the valid ones.
pub fn resolve(asked: &[String], catalogue: &[Entry]) -> Result<Vec<&'static str>, Failure> {
    let mut tools = Vec::new();
    for name in asked {
        let name = name.trim();
        if let Some(set) = ToolSet::named(name) {
            tools.extend(catalogue.iter().filter(|e| e.set == set).map(|e| e.name));
        } else if let Some(entry) = catalogue.iter().find(|entry| entry.name == name) {
            tools.push(entry.name);
        } else {
            let tools: Vec<&str> = catalogue.iter().map(|entry| entry.name).collect();
            return Err(Failure::Usage {
                message: format!(
                    "There is no MCP tool or tool set named {}. Tool sets: {}. Tools: {}.",
                    crate::mcp::untrusted::capped(name, 100),
                    names(&ToolSet::ALL),
                    joined(&tools)
                ),
                hint: Some(Hint::run("anima mcp permissions show")),
            });
        }
    }
    Ok(tools)
}

fn parse_state(name: &str) -> Result<State, Failure> {
    State::named(name).ok_or_else(|| saved_error(&format!("the state {name}"), "allow, ask, off"))
}

fn parse_sets(sets: &[String]) -> Result<Vec<ToolSet>, Failure> {
    sets.iter()
        .map(|name| {
            ToolSet::named(name)
                .ok_or_else(|| saved_error(&format!("the tool set {name}"), &names(&ToolSet::ALL)))
        })
        .collect()
}

fn saved_error(what: &str, valid: &str) -> Failure {
    Failure::Usage {
        message: format!(
            "The saved MCP permissions name {}, which does not exist. Valid: {valid}.",
            crate::mcp::untrusted::capped(what, 100)
        ),
        hint: Some(Hint::run("anima mcp permissions reset")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::catalogue;

    const UPDATE: &str = "anima_check_update";

    #[test]
    fn mcp_002_nothing_saved_is_read_tools_allowed_and_change_tools_off() {
        let none = Permissions::default();
        assert_eq!(none.state("anima_device_get", Kind::Read), State::Allow);
        assert_eq!(none.state(UPDATE, Kind::Change), State::Off);
        assert_eq!(none.choice(UPDATE, Kind::Change), None);
    }

    #[test]
    fn mcp_012_the_saved_form_round_trips_and_keeps_unknown_tool_names() {
        let mut permissions = Permissions {
            change: Some(State::Ask),
            ..Permissions::default()
        };
        permissions.choose(UPDATE, State::Allow);
        permissions.choose("anima_removed_later", State::Off);
        let saved = permissions.to_saved();
        assert_eq!(saved.change.as_deref(), Some("ask"));
        assert_eq!(saved.read, None);
        let text = serde_json::to_string(&saved).unwrap_or_default();
        assert!(
            !text.contains("toolsets") && !text.contains("write"),
            "{text}"
        );
        let back = Permissions::from_saved(&saved, &catalogue()).ok();
        assert_eq!(back, Some(permissions));
    }

    #[test]
    fn mcp_012_the_form_before_it_becomes_explicit_choices() {
        let old = McpPermissions {
            toolsets: vec!["devices".to_owned(), "checks".to_owned()],
            write: vec!["checks".to_owned()],
            ..McpPermissions::default()
        };
        let permissions = Permissions::from_saved(&old, &catalogue()).expect("valid");
        assert_eq!(permissions.state(UPDATE, Kind::Change), State::Allow);
        assert_eq!(
            permissions.state("anima_device_create", Kind::Change),
            State::Off
        );
        assert_eq!(
            permissions.state("anima_vpn_status", Kind::Read),
            State::Off
        );
        assert_eq!(
            permissions.state("anima_device_get", Kind::Read),
            State::Allow
        );
    }

    #[test]
    fn mcp_012_a_saved_name_that_does_not_exist_is_refused() {
        let wrong = McpPermissions {
            change: Some("sometimes".to_owned()),
            ..McpPermissions::default()
        };
        let error = Permissions::from_saved(&wrong, &catalogue()).expect_err("unknown state");
        assert!(error.to_string().contains("allow, ask, off"), "{error}");
    }

    #[test]
    fn mcp_014_a_bulk_choice_without_a_set_moves_the_default_and_clears_choices() {
        let catalogue = catalogue();
        let mut permissions = Permissions::default();
        permissions.choose(UPDATE, State::Allow);
        permissions.choose("anima_device_get", State::Off);
        let change = Scope {
            set: None,
            kind: Some(Kind::Change),
        };
        permissions.choose_all(State::Ask, change, &catalogue);
        assert_eq!(permissions.change, Some(State::Ask));
        assert_eq!(permissions.state(UPDATE, Kind::Change), State::Ask);
        assert_eq!(
            permissions.state("anima_device_get", Kind::Read),
            State::Off
        );
        assert_eq!(permissions.read, None);
    }

    #[test]
    fn mcp_014_a_bulk_choice_for_one_set_saves_each_of_its_tools() {
        let catalogue = catalogue();
        let mut permissions = Permissions::default();
        let checks = Scope {
            set: Some(ToolSet::Checks),
            kind: None,
        };
        permissions.choose_all(State::Allow, checks, &catalogue);
        assert_eq!(permissions.change, None);
        assert_eq!(permissions.state(UPDATE, Kind::Change), State::Allow);
        assert_eq!(
            permissions.state("anima_device_create", Kind::Change),
            State::Off
        );
    }

    #[test]
    fn mcp_014_names_are_tools_or_sets_and_unknown_names_list_the_valid_ones() {
        let catalogue = catalogue();
        let asked = vec!["vpn".to_owned(), UPDATE.to_owned()];
        let tools = resolve(&asked, &catalogue).expect("known");
        assert_eq!(
            tools,
            vec!["anima_vpn_status", "anima_vpn_approve_route", UPDATE]
        );
        let error = resolve(&["printers".to_owned()], &catalogue).expect_err("unknown");
        let text = error.to_string();
        assert!(
            text.contains("retention") && text.contains(UPDATE),
            "{text}"
        );
    }
}
