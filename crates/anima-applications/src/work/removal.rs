//! `application_remove` (contract K10 Section 6.4.8): stops and removes the containers and
//! the networks of one application that Anima installed.

use super::{Facts, free, managed};
use crate::{
    Plan, Refusal, Removal, Step,
    forms::{is_engine_name, project_of},
    inventory::{digest_of, project_is_protected},
    plan::Project,
};

/// The steps that remove what the engine has of `project`, by the project label, when the
/// folder of the application is gone.
fn by_the_label(project: &str, volumes: bool, facts: &Facts<'_>) -> Vec<Step> {
    let inventory = facts.inventory;
    let of_the_project = |label: &Option<String>| label.as_deref() == Some(project);
    let containers: Vec<&str> = inventory
        .containers
        .iter()
        .filter(|container| container.is_of(project) && container.is_sound())
        .map(|container| container.id.as_str())
        .collect();
    let networks: Vec<&str> = inventory
        .networks
        .iter()
        .filter(|network| of_the_project(&network.project) && !network.of_the_engine)
        .map(|network| network.name.as_str())
        .collect();
    let named: Vec<&str> = inventory
        .volumes
        .iter()
        .filter(|volume| volumes && of_the_project(&volume.project))
        .map(|volume| volume.name.as_str())
        .collect();
    let groups: [(&[&str], Vec<&str>); 3] = [
        (&["container", "rm", "--force"], containers),
        (&["network", "rm"], networks),
        (&["volume", "rm"], named),
    ];
    groups
        .into_iter()
        .map(|(verbs, names)| {
            (
                verbs,
                names
                    .into_iter()
                    .filter(|name| is_engine_name(name))
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, names)| !names.is_empty())
        .map(|(verbs, names)| Step::Run(facts.paths.docker(verbs, &names)))
        .collect()
}

/// The removal of each image that the program pulled for `application`. The engine refuses
/// to remove an image that a container uses; no step forces it. A refused removal does not
/// stop the plan. An image that a version of a different application holds is left out.
fn image_steps(application: &str, facts: &Facts<'_>) -> Vec<Step> {
    let installed = facts.records.installed(application);
    let pulled = installed.map_or_default(|installed| installed.pulled.as_slice());
    let held = |image: &&String| {
        digest_of(image).is_some_and(|digest| facts.records.holds_digest(digest, Some(application)))
    };
    pulled
        .iter()
        .filter(|image| is_pulled_name(image) && !held(image))
        .map(|image| Step::Attempt(facts.paths.docker(&["image", "rm"], &[image])))
        .collect()
}

/// The plan of `application_remove`. It deletes `files/` of the application in each case,
/// directly after the stop and also when the stop failed, and `binds/` only with `volumes`
/// and only after a stop that did not fail.
///
/// # Errors
/// `applications_off`; `not_at_machine` for `volumes` while "The hub may delete volumes" is
/// off; `not_adopted` when the program has no record of the application and the engine has
/// no container of its project; `protected`; `busy`.
pub fn removed(removal: &Removal, facts: &Facts<'_>) -> Result<Plan, Refusal> {
    managed(facts)?;
    if removal.volumes && !facts.settings.hub_may_delete_volumes {
        return Err(Refusal::NotAtMachine);
    }
    let project = project_of(&removal.application);
    let installed = facts.records.installed(&removal.application);
    let has_containers = facts
        .inventory
        .containers
        .iter()
        .any(|container| container.is_of(&project));
    if installed.is_none() && !has_containers {
        return Err(Refusal::NotAdopted);
    }
    if project_is_protected(&project, facts.settings, facts.inventory) {
        return Err(Refusal::Protected);
    }
    free(facts)?;
    let folders = facts.paths.application(&removal.application);
    let mut steps = if installed.is_some_and(|installed| installed.has_files) {
        let files = [folders.final_form()];
        let named = Project {
            name: &project,
            folder: &folders.files(),
            files: &files,
        };
        let verbs: &[&str] = if removal.volumes {
            &["down", "--volumes"]
        } else {
            &["down"]
        };
        vec![
            Step::Absent {
                path: folders.dot_env(),
            },
            Step::Run(facts.paths.compose(&named, verbs)),
        ]
    } else {
        by_the_label(&project, removal.volumes, facts)
    };
    // `files/` holds the environment file and the final form: it is deleted directly after
    // the stop, also when a step before failed (Section 6.4.8, "in each case").
    steps.push(Step::DeleteFolderInEachCase {
        path: folders.files(),
    });
    if removal.images {
        steps.extend(image_steps(&removal.application, facts));
    }
    if removal.volumes {
        steps.push(Step::DeleteFolder {
            path: folders.application().to_owned(),
        });
    }
    Ok(Plan { steps })
}

/// A name of an image with its digest, as the program records a pull: `name@sha256:…`.
pub(crate) fn is_pulled_name(image: &str) -> bool {
    image.split_once('@').is_some_and(|(name, digest)| {
        crate::forms::is_digest(digest)
            && name
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-/:".contains(&byte))
    })
}
