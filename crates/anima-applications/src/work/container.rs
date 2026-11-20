//! `container_action` (contract K10 Section 6.4.6) and `container_follow` (Section 6.4.11):
//! one container that Section 6.4.3 lets the type reach.

use super::{Facts, free, managed};
use crate::{
    Action, Plan, Refusal, Step,
    inventory::{is_installed, reached},
};

/// The plan of `container_action`.
///
/// # Errors
/// `applications_off`; `no_container`, `protected` and `not_adopted` (Section 6.4.3);
/// `start_not_permitted` for a start of a container that is neither of an application that
/// Anima installed nor has a restart policy other than `no`; `busy` for a start or a restart
/// while an item that changes Docker runs. A `stop` is never refused for `busy`.
pub fn acted(id: &str, action: Action, facts: &Facts<'_>) -> Result<Plan, Refusal> {
    managed(facts)?;
    let container = reached(id, facts.settings, facts.inventory, facts.records)?;
    // A restart of a container that does not run starts it, so it has the rule of a start.
    let starts = match action {
        Action::Start => true,
        Action::Restart => !container.running,
        Action::Stop => false,
    };
    if starts && !is_installed(container, facts.records) && !container.restarts {
        return Err(Refusal::StartNotPermitted);
    }
    if action != Action::Stop {
        free(facts)?;
    }
    let verb = match action {
        Action::Start => "start",
        Action::Stop => "stop",
        Action::Restart => "restart",
    };
    let command = facts.paths.docker(&["container", verb], &[&container.id]);
    Ok(Plan {
        steps: vec![Step::Run(command)],
    })
}

/// The plan of `container_follow`: the last `tail` lines of the log of one container, and
/// each later line. It changes nothing, so it is not refused for `busy`.
///
/// # Errors
/// `applications_off`; `no_container`, `protected` and `not_adopted` (Section 6.4.3).
pub fn followed(id: &str, tail: u64, facts: &Facts<'_>) -> Result<Plan, Refusal> {
    managed(facts)?;
    let container = reached(id, facts.settings, facts.inventory, facts.records)?;
    let tail = tail.min(1000).to_string();
    let verbs = ["container", "logs", "--follow", "--tail", &tail];
    let command = facts.paths.docker(&verbs, &[&container.id]);
    Ok(Plan {
        steps: vec![Step::Run(command)],
    })
}
