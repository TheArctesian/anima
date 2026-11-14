//! What the server does with each tool in this run (MCP-002, MCP-003, MCP-015): a pure
//! function of the catalogue, the token's kind, the saved permissions and the flags at start.
//! The flags only narrow, with one exception: `--allow-write` allows the change tools that
//! have no saved choice. A saved `off` or `ask` is never weakened by a flag.

use super::catalogue::{Kind, ToolSet};
use super::permissions::{Permissions, State};
use super::tools::Entry;
use crate::hub::TokenKind;

/// The flags of `anima mcp`, for one run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--toolsets`: every tool outside these sets is off.
    pub toolsets: Option<Vec<ToolSet>>,
    /// `--allow-write [sets]`: the change tools of these sets (`Some(empty)`: every set) that
    /// have no saved choice are allowed.
    pub allow_write: Option<Vec<ToolSet>>,
    /// `--read-only`: every change tool is off.
    pub read_only: bool,
}

/// One tool and what the server does with it.
#[derive(Clone, Debug)]
pub struct Row {
    pub entry: Entry,
    /// `off`: not listed and refused if called.
    pub state: State,
}

/// Each tool of `catalogue` with its state for this run. With no token kind (`permissions
/// show`) the tools that need sign-in keep their saved state.
#[must_use]
pub fn select(
    catalogue: Vec<Entry>,
    token: Option<TokenKind>,
    saved: &Permissions,
    flags: &Flags,
) -> Vec<Row> {
    catalogue
        .into_iter()
        .map(|entry| {
            let state = state(&entry, token, saved, flags);
            Row { entry, state }
        })
        .collect()
}

fn state(entry: &Entry, token: Option<TokenKind>, saved: &Permissions, flags: &Flags) -> State {
    let kind = entry.kind();
    let choice = saved.choice(entry.name, kind);
    let raised = kind == Kind::Change
        && choice.is_none()
        && flags
            .allow_write
            .as_ref()
            .is_some_and(|sets| sets.is_empty() || sets.contains(&entry.set));
    let outside = flags
        .toolsets
        .as_ref()
        .is_some_and(|sets| !sets.contains(&entry.set));
    let barred = token == Some(TokenKind::ApiKey) && entry.definition.access.needs_sign_in();
    if outside || barred || (flags.read_only && kind == Kind::Change) {
        State::Off
    } else if raised {
        State::Allow
    } else {
        choice.unwrap_or_else(|| State::built_in(kind))
    }
}

/// The plain picture of `rows`, for the server's instructions and `permissions show`.
#[must_use]
pub fn statement(rows: &[Row]) -> String {
    let sets: Vec<&str> = ToolSet::ALL
        .into_iter()
        .filter(|set| {
            rows.iter()
                .any(|row| row.entry.set == *set && row.state != State::Off)
        })
        .map(ToolSet::name)
        .collect();
    format!(
        "Tool sets on: {}. Change tools allowed: {}. Tools that ask the operator before each \
         call: {}. Off, not listed: {}. The hub token is still the final limit: an API key \
         with read scope cannot change anything, and the tools that need a login token \
         (device update and delete, release create, build cancel, VPN route approval, \
         retention set, extension set, source account removal) are never listed for an API key.",
        super::catalogue::joined(&sets),
        grouped(rows, |row| {
            row.state == State::Allow && row.entry.kind() == Kind::Change
        }),
        grouped(rows, |row| row.state == State::Ask),
        grouped(rows, |row| row.state == State::Off),
    )
}

/// The rows that `matches`, by set: `checks (change tools)` or `checks (all)` when the whole
/// group matches, else the tool names.
fn grouped(rows: &[Row], matches: impl Fn(&Row) -> bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    for set in ToolSet::ALL {
        let of_set: Vec<&Row> = rows.iter().filter(|row| row.entry.set == set).collect();
        let hits: Vec<&Row> = of_set.iter().copied().filter(|row| matches(row)).collect();
        let changes = of_set
            .iter()
            .filter(|row| row.entry.kind() == Kind::Change)
            .count();
        let only_changes = hits.iter().all(|row| row.entry.kind() == Kind::Change);
        if hits.is_empty() {
            continue;
        }
        if only_changes && hits.len() == changes {
            parts.push(format!("{} (change tools)", set.name()));
        } else if hits.len() == of_set.len() {
            parts.push(format!("{} (all)", set.name()));
        } else {
            parts.extend(hits.iter().map(|row| row.entry.name.to_owned()));
        }
    }
    super::catalogue::joined(&parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::catalogue;

    const UPDATE: &str = "anima_check_update";
    const DELETE: &str = "anima_check_delete";

    fn state_of(rows: &[Row], name: &str) -> State {
        rows.iter()
            .find(|row| row.entry.name == name)
            .map_or(State::Off, |row| row.state)
    }

    fn run(token: TokenKind, saved: &Permissions, flags: &Flags) -> Vec<Row> {
        select(catalogue(), Some(token), saved, flags)
    }

    #[test]
    fn mcp_002_nothing_saved_and_no_flag_is_read_only() {
        let rows = run(TokenKind::Login, &Permissions::default(), &Flags::default());
        for row in &rows {
            let expected = State::built_in(row.entry.kind());
            assert_eq!(row.state, expected, "{}", row.entry.name);
        }
        let text = statement(&rows);
        assert!(text.contains("Change tools allowed: none."), "{text}");
        assert!(text.contains("checks (change tools)"), "{text}");
    }

    #[test]
    fn mcp_015_allow_write_raises_only_tools_with_no_saved_choice() {
        let mut saved = Permissions::default();
        saved.choose(DELETE, State::Ask);
        saved.choose("anima_check_set_paused", State::Off);
        let flags = Flags {
            allow_write: Some(vec![ToolSet::Checks]),
            ..Flags::default()
        };
        let rows = run(TokenKind::Login, &saved, &flags);
        assert_eq!(state_of(&rows, UPDATE), State::Allow);
        assert_eq!(state_of(&rows, DELETE), State::Ask);
        assert_eq!(state_of(&rows, "anima_check_set_paused"), State::Off);
        assert_eq!(state_of(&rows, "anima_device_create"), State::Off);
        let every = Flags {
            allow_write: Some(Vec::new()),
            ..Flags::default()
        };
        let rows = run(TokenKind::Login, &saved, &every);
        assert_eq!(state_of(&rows, "anima_device_create"), State::Allow);
    }

    #[test]
    fn mcp_015_a_saved_change_default_counts_as_a_choice() {
        let saved = Permissions {
            change: Some(State::Ask),
            ..Permissions::default()
        };
        let flags = Flags {
            allow_write: Some(Vec::new()),
            ..Flags::default()
        };
        let rows = run(TokenKind::Login, &saved, &flags);
        assert_eq!(state_of(&rows, UPDATE), State::Ask);
    }

    #[test]
    fn mcp_015_read_only_and_toolsets_only_narrow() {
        let mut saved = Permissions {
            change: Some(State::Allow),
            ..Permissions::default()
        };
        saved.choose("anima_vpn_status", State::Ask);
        let read_only = Flags {
            read_only: true,
            ..Flags::default()
        };
        let rows = run(TokenKind::Login, &saved, &read_only);
        assert_eq!(state_of(&rows, UPDATE), State::Off);
        assert_eq!(state_of(&rows, "anima_vpn_status"), State::Ask);
        let checks = Flags {
            toolsets: Some(vec![ToolSet::Checks, ToolSet::Vpn]),
            ..Flags::default()
        };
        let rows = run(TokenKind::Login, &saved, &checks);
        assert_eq!(state_of(&rows, UPDATE), State::Allow);
        assert_eq!(state_of(&rows, "anima_vpn_status"), State::Ask);
        assert_eq!(state_of(&rows, "anima_device_get"), State::Off);
    }

    #[test]
    fn mcp_003_an_api_key_never_gets_a_sign_in_tool_whatever_is_saved() {
        let mut saved = Permissions {
            change: Some(State::Allow),
            ..Permissions::default()
        };
        saved.choose("anima_retention_set", State::Allow);
        let rows = run(TokenKind::ApiKey, &saved, &Flags::default());
        for row in &rows {
            if row.entry.definition.access.needs_sign_in() {
                assert_eq!(row.state, State::Off, "{}", row.entry.name);
            }
        }
        assert_eq!(state_of(&rows, UPDATE), State::Allow);
        let login = run(TokenKind::Login, &saved, &Flags::default());
        assert_eq!(state_of(&login, "anima_retention_set"), State::Allow);
        let shown = select(catalogue(), None, &saved, &Flags::default());
        assert_eq!(state_of(&shown, "anima_retention_set"), State::Allow);
    }

    #[test]
    fn mcp_016_the_statement_names_sets_asking_tools_and_what_is_off() {
        let mut saved = Permissions::default();
        saved.choose(DELETE, State::Ask);
        saved.choose(UPDATE, State::Allow);
        let flags = Flags {
            toolsets: Some(vec![ToolSet::Devices, ToolSet::Checks]),
            ..Flags::default()
        };
        let text = statement(&run(TokenKind::Login, &saved, &flags));
        assert!(text.contains("Tool sets on: devices, checks."), "{text}");
        assert!(
            text.contains("Change tools allowed: anima_check_update."),
            "{text}"
        );
        assert!(
            text.contains("before each call: anima_check_delete."),
            "{text}"
        );
        assert!(text.contains("events (all)"), "{text}");
        assert!(text.contains("devices (change tools)"), "{text}");
    }
}
