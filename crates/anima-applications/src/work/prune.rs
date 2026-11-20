//! `docker_prune` (contract K10 Section 6.4.10): removes stopped containers, networks that
//! no container uses, and images that no container uses. The program names each thing that
//! it removes: it never gives the engine a prune command, which cannot leave out what the
//! list "Protected", an adopted project or `keep` holds.

use super::{Facts, free, managed};
use crate::{
    Container, Image, Network, Plan, Pruned, Refusal, Step,
    final_form::image_by_digest,
    forms::{is_digest, is_engine_name},
    inventory::{container_is_protected, is_installed},
};

fn label_is_adopted(label: Option<&str>, facts: &Facts<'_>) -> bool {
    label.is_some_and(|project| facts.settings.adopted_project(project).is_some())
}

fn label_is_protected(label: Option<&str>, facts: &Facts<'_>) -> bool {
    label.is_some_and(|project| facts.settings.protects(project))
}

/// A stopped container of an application that Anima installed; with "All containers" on,
/// each stopped container. Never one of the list "Protected" or of an adopted project.
fn container_goes(container: &Container, facts: &Facts<'_>) -> bool {
    let reached = facts.settings.all_containers || is_installed(container, facts.records);
    reached
        && !container.running
        && container.is_sound()
        && !container_is_protected(container, facts.settings)
        && !label_is_adopted(container.project.as_deref(), facts)
}

/// A network that no container uses, of the project of an application that Anima installed;
/// with "All containers" on, each such network that the engine did not make itself.
fn network_goes(network: &Network, facts: &Facts<'_>) -> bool {
    let label = network.project.as_deref();
    let installed = label.is_some_and(|project| facts.records.has_project(project));
    (facts.settings.all_containers || installed)
        && !network.used
        && !network.of_the_engine
        && is_engine_name(&network.name)
        && !facts.settings.protects(&network.name)
        && !label_is_protected(label, facts)
        && !label_is_adopted(label, facts)
}

/// An image that no container uses and that the program pulled for an application; with
/// "All containers" on, also each image with no tag. Never an image that a version that
/// runs or a kept version holds.
fn image_goes(image: &Image, facts: &Facts<'_>) -> bool {
    let applications = &facts.records.applications;
    let named = |name: &String| image.pulled_as.contains(name);
    let pulled = applications
        .iter()
        .any(|installed| installed.pulled.iter().any(named));
    let held = applications.iter().any(|installed| {
        let kept = installed.kept.iter().flatten();
        let mut versions = installed.current.iter().chain(kept);
        versions.any(|one| named(&image_by_digest(&one.image, &one.digest)))
    });
    let reached = pulled || (facts.settings.all_containers && image.dangling);
    reached && !image.used && !held && is_digest(&image.id)
}

/// The plan of `docker_prune` for the members `what`, in the order containers, networks,
/// images, so that an image of a removed container is not held by it.
///
/// # Errors
/// `applications_off`; `busy`.
pub fn pruned(what: &[Pruned], facts: &Facts<'_>) -> Result<Plan, Refusal> {
    managed(facts)?;
    free(facts)?;
    let inventory = facts.inventory;
    let mut steps = Vec::new();
    let mut run = |verbs: &[&str], names: Vec<&str>| {
        if !names.is_empty() {
            steps.push(Step::Run(facts.paths.docker(verbs, &names)));
        }
    };
    if what.contains(&Pruned::Containers) {
        let goes = inventory
            .containers
            .iter()
            .filter(|one| container_goes(one, facts));
        run(
            &["container", "rm"],
            goes.map(|one| one.id.as_str()).collect(),
        );
    }
    if what.contains(&Pruned::Networks) {
        let goes = inventory
            .networks
            .iter()
            .filter(|one| network_goes(one, facts));
        run(
            &["network", "rm"],
            goes.map(|one| one.name.as_str()).collect(),
        );
    }
    if what.contains(&Pruned::Images) {
        let goes = inventory.images.iter().filter(|one| image_goes(one, facts));
        run(&["image", "rm"], goes.map(|one| one.id.as_str()).collect());
    }
    Ok(Plan { steps })
}
