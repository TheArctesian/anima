//! `docker_prune` (K10 Section 6.4.10).

use super::{DIGEST, FIRST, OF_ADOPTED, OF_ANIMA, OF_THE_OWNER, PROJECT, World, commands, pulled};
use crate::{Image, Network, Pruned, Refusal, work::prune::pruned};

const OLD: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const RUNS: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const KEPT: &str = "sha256:3333333333333333333333333333333333333333333333333333333333333333";
const OF_THE_OWNER_IMAGE: &str =
    "sha256:4444444444444444444444444444444444444444444444444444444444444444";
const ALL: [Pruned; 3] = [Pruned::Containers, Pruned::Networks, Pruned::Images];

fn network(name: &str, project: Option<&str>) -> Network {
    Network {
        name: name.to_owned(),
        project: project.map(str::to_owned),
        used: false,
        of_the_engine: false,
    }
}

fn image(id: &str, pulled_as: &str, dangling: bool) -> Image {
    Image {
        id: id.to_owned(),
        pulled_as: vec![pulled_as.to_owned()],
        used: false,
        dangling,
    }
}

/// A machine where each container is stopped, with networks and images of each owner.
fn stopped() -> World {
    let mut world = World::new();
    for container in &mut world.inventory.containers {
        container.running = false;
    }
    world.inventory.networks = vec![
        network("anima-net", Some(PROJECT)),
        network("tracks_default", Some("tracks")),
        network("lab", None),
        Network {
            of_the_engine: true,
            ..network("bridge", None)
        },
    ];
    let old =
        "example/tracks@sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let kept_digest = "sha256:9999999999999999999999999999999999999999999999999999999999999999";
    world.inventory.images = vec![
        image(OLD, old, false),
        image(RUNS, &format!("example/tracks@{DIGEST}"), false),
        image(KEPT, &format!("example/tracks@{kept_digest}"), false),
        image(OF_THE_OWNER_IMAGE, "example/other@sha256:abc", true),
    ];
    let installed = &mut world.records.applications[0];
    installed.pulled = vec![
        old.to_owned(),
        format!("example/tracks@{DIGEST}"),
        format!("example/tracks@{kept_digest}"),
    ];
    installed.kept = Some(vec![crate::Pulled {
        digest: kept_digest.to_owned(),
        ..pulled("app", "example/tracks:0")
    }]);
    world
}

/// The arguments of each command of the plan for `what`, after the first four.
fn removed(world: &World, what: &[Pruned]) -> Vec<String> {
    let plan = pruned(what, &world.facts()).expect("a plan");
    commands(&plan)
        .iter()
        .map(|command| {
            assert_eq!(command.arguments[..4], FIRST);
            command.arguments[4..].join(" ")
        })
        .collect()
}

#[test]
fn k10_6_4_10_a_prune_removes_only_what_is_of_an_application_that_anima_installed() {
    let world = stopped();
    assert_eq!(
        removed(&world, &ALL),
        [
            format!("container rm -- {OF_ANIMA}"),
            "network rm -- anima-net".to_owned(),
            format!("image rm -- {OLD}"),
        ]
    );
    assert_eq!(
        removed(&world, &[Pruned::Networks]),
        ["network rm -- anima-net"]
    );
}

#[test]
fn k10_6_4_10_a_prune_never_removes_what_runs_what_is_used_or_what_keep_holds() {
    let mut world = stopped();
    world.inventory.containers[0].running = true;
    world.inventory.networks[0].used = true;
    world.inventory.images[0].used = true;
    assert_eq!(removed(&world, &ALL), Vec::<String>::new());
    // The image of the kept version and of the version that runs stay, used or not.
    let world = stopped();
    let images = removed(&world, &[Pruned::Images]).join(" ");
    assert!(!images.contains(KEPT) && !images.contains(RUNS));
}

#[test]
fn k10_6_4_10_with_all_containers_on_the_members_mean_what_the_prune_commands_remove() {
    let mut world = stopped();
    world.settings.all_containers = true;
    assert_eq!(
        removed(&world, &ALL),
        [
            // Never a container and never a network of an adopted project (R-211).
            format!("container rm -- {OF_ANIMA} {OF_THE_OWNER}"),
            "network rm -- anima-net lab".to_owned(),
            format!("image rm -- {OLD} {OF_THE_OWNER_IMAGE}"),
        ]
    );
    assert!(!removed(&world, &ALL).join(" ").contains(OF_ADOPTED));
}

#[test]
fn k10_6_4_10_a_prune_never_removes_a_thing_of_the_list_protected() {
    let mut world = stopped();
    world.settings.all_containers = true;
    world.settings.protected = vec!["backup".to_owned(), "lab".to_owned(), PROJECT.to_owned()];
    assert_eq!(
        removed(&world, &[Pruned::Containers, Pruned::Networks]),
        Vec::<String>::new()
    );
}

#[test]
fn k10_6_4_10_a_prune_is_refused_while_an_item_that_changes_docker_runs() {
    let mut world = stopped();
    world.busy = true;
    assert_eq!(pruned(&ALL, &world.facts()), Err(Refusal::Busy));
}

#[test]
fn k10_6_4_10_an_image_that_keep_holds_stays_when_the_engine_writes_a_different_name() {
    let mut world = World::new();
    world.settings.all_containers = true;
    let kept_digest = "sha256:5555555555555555555555555555555555555555555555555555555555555555";
    world.records.applications[0].kept = Some(vec![crate::Pulled {
        digest: kept_digest.to_owned(),
        ..pulled("app", "docker.io/library/nginx:1")
    }]);
    // The engine writes the short name; the file named the long one.
    world.inventory.images = vec![image(OLD, &format!("nginx@{kept_digest}"), true)];
    assert_eq!(removed(&world, &[Pruned::Images]), Vec::<String>::new());
    // The same image with a digest that no version holds goes.
    world.records.applications[0].kept = None;
    assert_eq!(
        removed(&world, &[Pruned::Images]),
        [format!("image rm -- {OLD}")]
    );
}
