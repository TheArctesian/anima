//! `adopted_apply` (K10 Section 6.4.7).

use super::{FIRST, World, arguments, commands};
use crate::{AdoptedAction, Refusal, work::adopted::applied};

const NAMED: [&str; 6] = [
    "--project-name",
    "tracks",
    "--project-directory",
    "/srv/tracks",
    "--file",
    "/srv/tracks/compose.yaml",
];

#[test]
fn k10_6_4_7_an_adopted_project_is_started_from_its_recorded_paths() {
    let mut world = World::new();
    // A label of a container at the time of the item is never used.
    world.inventory.containers[1].folder = Some("/tmp/other".to_owned());
    let plan = applied("tracks", (AdoptedAction::Up, false), &world.facts()).expect("adopted");
    let arguments = arguments(&plan);
    assert_eq!(arguments[..4], FIRST);
    assert_eq!(arguments[4..10], NAMED);
    assert_eq!(arguments[10..], ["up", "-d", "--no-build"]);
    let command = commands(&plan)[0];
    assert_eq!(
        command.program,
        "/usr/libexec/docker/cli-plugins/docker-compose"
    );
    assert_eq!(command.folder.as_deref(), Some("/srv/tracks"));
    // The plan writes no file: the program never changes a file of an adopted project.
    assert_eq!(plan.steps.len(), 1);
}

#[test]
fn k10_6_4_7_up_pulls_only_when_pull_is_true_and_there_is_no_down() {
    let world = World::new();
    let verbs = |action, pull| {
        let plan = applied("tracks", (action, pull), &world.facts())?;
        Ok::<_, Refusal>(arguments(&plan)[10..].join(" "))
    };
    assert_eq!(
        verbs(AdoptedAction::Up, true).as_deref(),
        Ok("up -d --no-build --pull always")
    );
    assert_eq!(verbs(AdoptedAction::Stop, false).as_deref(), Ok("stop"));
    assert_eq!(
        verbs(AdoptedAction::Restart, false).as_deref(),
        Ok("restart")
    );
    assert_eq!(
        verbs(AdoptedAction::Stop, true),
        Err(Refusal::MemberUnknown)
    );
    assert_eq!(
        verbs(AdoptedAction::Restart, true),
        Err(Refusal::MemberUnknown)
    );
}

#[test]
fn k10_6_4_3_a_project_that_is_not_adopted_is_refused() {
    let mut world = World::new();
    let up =
        |world: &World, project: &str| applied(project, (AdoptedAction::Up, false), &world.facts());
    assert_eq!(up(&world, "other"), Err(Refusal::NotAdopted));
    // "All containers" does not adopt a project.
    world.settings.all_containers = true;
    assert_eq!(up(&world, "other"), Err(Refusal::NotAdopted));
    // A project whose name starts with `anima-` cannot be adopted, also when a settings
    // value names it.
    world.settings.adopted[0].name = super::PROJECT.to_owned();
    assert_eq!(up(&world, super::PROJECT), Err(Refusal::NotAdopted));
}

#[test]
fn k10_6_4_3_a_protected_project_or_a_project_with_a_protected_container_is_refused() {
    let mut world = World::new();
    let stop = |world: &World| applied("tracks", (AdoptedAction::Stop, false), &world.facts());
    world.settings.protected = vec!["tracks".to_owned()];
    assert_eq!(stop(&world), Err(Refusal::Protected));
    world.settings.protected = vec!["tracks-web-1".to_owned()];
    assert_eq!(stop(&world), Err(Refusal::Protected));
    world.settings.protected = vec!["backup".to_owned()];
    assert!(stop(&world).is_ok());
}

#[test]
fn k10_6_4_1_an_adopted_apply_is_refused_while_an_item_that_changes_docker_runs() {
    let mut world = World::new();
    world.busy = true;
    let stop = applied("tracks", (AdoptedAction::Stop, false), &world.facts());
    assert_eq!(stop, Err(Refusal::Busy));
}
