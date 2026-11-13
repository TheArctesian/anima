//! Whether a change to the saved permissions gives an agent more than it had (MCP-014): a
//! tool, or a kind's default, that moves toward `allow` (`off` to `ask` or `allow`, `ask` to
//! `allow`). Only a person makes such a change: at a terminal, or in the desktop app's
//! Settings. A change toward `off` is made anywhere. An agent's shell has no terminal on
//! standard input, so it cannot widen its own access through `anima mcp permissions`.

use super::catalogue::Kind;
use super::permissions::{Permissions, State};
use super::tools::Entry;
use crate::failure::Failure;

/// Whether `after` allows more than `before`: a tool of `catalogue` or a kind's default (which
/// tools added later take) has a state nearer to `allow`.
#[must_use]
pub fn widens(before: &Permissions, after: &Permissions, catalogue: &[Entry]) -> bool {
    let tool = catalogue.iter().any(|entry| {
        let kind = entry.kind();
        after.state(entry.name, kind) < before.state(entry.name, kind)
    });
    let default = [Kind::Read, Kind::Change]
        .into_iter()
        .any(|kind| default(after, kind) < default(before, kind));
    tool || default
}

/// Lets the change from `before` to `after` go on when it does not widen, or when standard
/// input is a terminal. `--yes` has no say here.
///
/// # Errors
/// [`Failure::Usage`] (exit 2) for a widening without a terminal on standard input.
pub fn by_a_person(
    stdin_terminal: bool,
    before: &Permissions,
    after: &Permissions,
    catalogue: &[Entry],
) -> Result<(), Failure> {
    if stdin_terminal || !widens(before, after, catalogue) {
        return Ok(());
    }
    Err(Failure::Usage {
        message: "This change gives an MCP agent more access (a tool or a default moves toward \
                  allow), so a person must make it: at a terminal, or in the Anima desktop \
                  app's Settings. Standard input is not a terminal, and --yes does not change \
                  that. Changes toward off work anywhere."
            .to_owned(),
        hint: None,
    })
}

/// The state a kind's tools without their own choice take.
fn default(permissions: &Permissions, kind: Kind) -> State {
    let saved = match kind {
        Kind::Read => permissions.read,
        Kind::Change => permissions.change,
    };
    saved.unwrap_or_else(|| State::built_in(kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::catalogue::ToolSet;
    use crate::mcp::permissions::Scope;
    use crate::mcp::tools::catalogue;

    const DELETE: &str = "anima_check_delete";
    const READ: &str = "anima_vpn_status";

    fn after(change: impl Fn(&mut Permissions)) -> (Permissions, Permissions) {
        let before = Permissions::default();
        let mut after = before.clone();
        change(&mut after);
        (before, after)
    }

    #[test]
    fn mcp_014_f4_each_move_toward_allow_widens() {
        let catalogue = catalogue();
        for (from, to) in [
            (State::Off, State::Ask),
            (State::Off, State::Allow),
            (State::Ask, State::Allow),
        ] {
            let mut before = Permissions::default();
            before.choose(DELETE, from);
            let mut after = before.clone();
            after.choose(DELETE, to);
            assert!(widens(&before, &after, &catalogue), "{from:?} to {to:?}");
            assert!(!widens(&after, &before, &catalogue), "{to:?} to {from:?}");
        }
    }

    #[test]
    fn mcp_014_f4_a_kind_default_toward_allow_widens_even_with_every_tool_chosen() {
        let catalogue = catalogue();
        let mut before = Permissions::default();
        for entry in &catalogue {
            before.choose(entry.name, State::Off);
        }
        let mut after = before.clone();
        after.change = Some(State::Ask);
        assert!(widens(&before, &after, &catalogue));
        after.change = Some(State::Off);
        assert!(!widens(&before, &after, &catalogue));
    }

    #[test]
    fn mcp_014_f4_the_command_forms_widen_or_narrow_as_expected() {
        let catalogue = catalogue();
        let checks = Scope {
            set: Some(ToolSet::Checks),
            kind: None,
        };
        let reads = Scope {
            set: None,
            kind: Some(Kind::Read),
        };
        let (before, all) = after(|p| p.choose_all(State::Allow, Scope::default(), &catalogue));
        assert!(widens(&before, &all, &catalogue), "--allow-all");
        let (before, set) = after(|p| p.choose_all(State::Ask, checks, &catalogue));
        assert!(
            widens(&before, &set, &catalogue),
            "--ask-all --toolset checks"
        );
        let (before, same) = after(|p| p.choose_all(State::Allow, reads, &catalogue));
        assert!(
            !widens(&before, &same, &catalogue),
            "read tools are allowed already"
        );
        let (before, off) = after(|p| p.choose(READ, State::Off));
        assert!(!widens(&before, &off, &catalogue), "--off");
        assert!(widens(&off, &before, &catalogue), "reset after --off");
    }

    #[test]
    fn mcp_014_f4_only_a_terminal_may_widen_and_anyone_may_narrow() {
        let catalogue = catalogue();
        let (before, wider) = after(|p| p.choose(DELETE, State::Allow));
        let refused = by_a_person(false, &before, &wider, &catalogue).expect_err("no terminal");
        assert_eq!(refused.exit_code(), 2);
        let text = refused.to_string();
        assert!(
            text.contains("terminal") && text.contains("Settings"),
            "{text}"
        );
        assert!(by_a_person(true, &before, &wider, &catalogue).is_ok());
        assert!(by_a_person(false, &wider, &before, &catalogue).is_ok());
    }
}
