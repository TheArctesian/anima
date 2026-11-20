//! `adopted_apply` (contract K10 Section 6.4.7): a Compose project that Anima did not install
//! and that the owner adopted, started from its recorded paths.

use super::{Facts, free, managed};
use crate::{AdoptedAction, Plan, Refusal, Step, inventory::project_is_protected, plan::Project};

/// The plan of `adopted_apply` for `(action, pull)`. It uses the recorded files and the
/// recorded folder, never a label of a container, and it changes no file of the project
/// (R-211).
///
/// # Errors
/// `applications_off`; `member_unknown` for `pull` with an action other than `up`;
/// `not_adopted` for a project that is not in the list "Adopted projects", and for each
/// name that starts with `anima-`; `protected`; `busy`.
pub fn applied(
    project: &str,
    (action, pull): (AdoptedAction, bool),
    facts: &Facts<'_>,
) -> Result<Plan, Refusal> {
    managed(facts)?;
    if pull && action != AdoptedAction::Up {
        return Err(Refusal::MemberUnknown);
    }
    let adopted = facts
        .settings
        .adopted_project(project)
        .filter(|adopted| !adopted.name.starts_with("anima-"))
        .ok_or(Refusal::NotAdopted)?;
    if project_is_protected(project, facts.settings, facts.inventory) {
        return Err(Refusal::Protected);
    }
    free(facts)?;
    let verbs: &[&str] = match (action, pull) {
        (AdoptedAction::Up, true) => &["up", "-d", "--no-build", "--pull", "always"],
        (AdoptedAction::Up, false) => &["up", "-d", "--no-build"],
        (AdoptedAction::Stop, _) => &["stop"],
        (AdoptedAction::Restart, _) => &["restart"],
    };
    let named = Project {
        name: &adopted.name,
        folder: &adopted.folder,
        files: &adopted.files,
    };
    Ok(Plan {
        steps: vec![Step::Run(facts.paths.compose(&named, verbs))],
    })
}
