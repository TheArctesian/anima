//! `application_back` (contract K10 Section 6.4.9): brings one application back to the kept
//! version, by the six steps of Section 6.4.4 of today, with the kept digests and no pull.

use compose_rules::Rule;

use super::{
    Facts,
    apply::{
        Planned, Prepared, folder_steps, in_place_steps, project_checked, sha256, version_steps,
    },
    managed,
};
use crate::{Refusal, Step};

/// The kept Compose file and the kept environment file, as the program reads them from
/// `files/` of the application. `{:?}` shows only their lengths.
#[derive(Clone, Copy)]
pub struct Kept<'a> {
    pub compose: &'a [u8],
    pub environment: &'a [u8],
}

impl std::fmt::Debug for Kept<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Kept")
            .field("compose_bytes", &self.compose.len())
            .field("environment_bytes", &self.environment.len())
            .finish()
    }
}

/// The check of `application_back` before the program reads the kept files.
///
/// # Errors
/// `applications_off`; `nothing_kept` when the application has no kept version, also after
/// an earlier `application_back`; `protected`, `name_taken` and `busy` as for an apply.
pub fn accepted(application: &str, facts: &Facts<'_>) -> Result<(), Refusal> {
    managed(facts)?;
    let installed = facts.records.installed(application);
    if installed
        .and_then(|installed| installed.kept.as_ref())
        .is_none()
    {
        return Err(Refusal::NothingKept);
    }
    project_checked(application, facts)
}

/// The plan of `application_back`: the final form made again from the kept files by the
/// rules of today, started with the kept digests. It sets no tag on an image. When the
/// start did not fail, the kept version is deleted, so that a second `application_back` is
/// refused until the next `application_apply`, and the kept records become the current ones.
///
/// # Errors
/// Each refusal of [`accepted`]; `file_not_permitted` and `not_at_machine` when a kept file
/// breaks a rule of today; `no_digest` for a service with no kept digest.
pub fn prepared(application: &str, kept: &Kept<'_>, facts: &Facts<'_>) -> Result<Planned, Refusal> {
    accepted(application, facts)?;
    let installed = facts.records.installed(application);
    let digests = installed
        .and_then(|installed| installed.kept.clone())
        .ok_or(Refusal::NothingKept)?;
    let compose =
        std::str::from_utf8(kept.compose).map_err(|_| Refusal::file(Rule::YamlNotPermitted))?;
    let variables = std::str::from_utf8(kept.environment)
        .map_err(|_| Refusal::file(Rule::VariableNotPermitted))?;
    let version = Prepared {
        application: application.to_owned(),
        keep: false,
        checked: compose_rules::check(compose, variables, &facts.settings.of_a_file())?,
        compose: kept.compose.to_vec(),
        environment: kept.environment.to_vec(),
        known: digests,
        pulls: Vec::new(),
    };
    let (started, form) = version_steps(&version, &version.known, facts)?;
    let folders = facts.paths.application(application);
    let mut steps = folder_steps(&folders, &form.binds);
    steps.extend(started);
    steps.extend(folders.kept().into_iter().map(|path| Step::Delete { path }));
    steps.extend(in_place_steps(&folders));
    Ok(Planned {
        plan: crate::Plan { steps },
        final_form_sha256: sha256(form.text.as_bytes()),
        holds_volumes: form.holds_volumes,
    })
}
