//! `container_action` and `container_follow`, and which containers a type reaches
//! (K10 Sections 6.4.3, 6.4.6 and 6.4.11).

use super::{FIRST, OF_ADOPTED, OF_ANIMA, OF_THE_OWNER, World, arguments, commands};
use crate::{
    Action, Refusal,
    work::container::{acted, followed},
};

fn stop(world: &World, id: &str) -> Result<Vec<String>, Refusal> {
    acted(id, Action::Stop, &world.facts())
        .map(|plan| arguments(&plan).into_iter().map(str::to_owned).collect())
}

#[test]
fn k10_6_4_3_a_container_of_anima_or_of_an_adopted_project_is_reached() {
    let world = World::new();
    for id in [OF_ANIMA, OF_ADOPTED] {
        let arguments = stop(&world, id).expect("reached");
        assert_eq!(arguments[..4], FIRST);
        assert_eq!(arguments[4..], ["container", "stop", "--", id]);
    }
}

#[test]
fn k10_6_4_3_each_other_container_is_refused_unless_all_containers_is_on() {
    let mut world = World::new();
    assert_eq!(stop(&world, OF_THE_OWNER), Err(Refusal::NotAdopted));
    // A project label that names an application of which the program has no record.
    world.inventory.containers[2].project =
        Some("anima-ffffffffffffffffffffffffffffffff".to_owned());
    assert_eq!(stop(&world, OF_THE_OWNER), Err(Refusal::NotAdopted));
    world.settings.all_containers = true;
    assert!(stop(&world, OF_THE_OWNER).is_ok());
}

#[test]
fn k10_6_4_6_an_identifier_that_the_engine_does_not_have_is_refused() {
    let world = World::new();
    let unknown = "d".repeat(64);
    assert_eq!(stop(&world, &unknown), Err(Refusal::NoContainer));
    // A name and a short identifier are never looked for.
    assert_eq!(stop(&world, "anima-app-1"), Err(Refusal::NoContainer));
    assert_eq!(stop(&world, &OF_ANIMA[..12]), Err(Refusal::NoContainer));
}

#[test]
fn k10_6_4_3_a_container_or_a_project_of_the_list_protected_is_refused_by_each_type() {
    let mut world = World::new();
    world.settings.all_containers = true;
    world.settings.protected = vec!["backup".to_owned(), "tracks".to_owned()];
    for id in [OF_THE_OWNER, OF_ADOPTED] {
        for action in [Action::Start, Action::Stop, Action::Restart] {
            assert_eq!(acted(id, action, &world.facts()), Err(Refusal::Protected));
        }
        assert_eq!(followed(id, 10, &world.facts()), Err(Refusal::Protected));
    }
    assert!(stop(&world, OF_ANIMA).is_ok());
}

#[test]
fn k10_6_4_6_a_start_is_only_for_a_container_of_anima_or_one_with_a_restart_policy() {
    let mut world = World::new();
    world.settings.all_containers = true;
    for container in &mut world.inventory.containers {
        container.running = false;
    }
    let start = |world: &World, id: &str| acted(id, Action::Start, &world.facts());
    assert!(start(&world, OF_ANIMA).is_ok());
    assert_eq!(start(&world, OF_ADOPTED), Err(Refusal::StartNotPermitted));
    assert_eq!(start(&world, OF_THE_OWNER), Err(Refusal::StartNotPermitted));
    // A restart of a container that does not run is a start.
    let restart = acted(OF_THE_OWNER, Action::Restart, &world.facts());
    assert_eq!(restart, Err(Refusal::StartNotPermitted));
    world.inventory.containers[2].restarts = true;
    let plan = start(&world, OF_THE_OWNER).expect("a restart policy other than no");
    assert_eq!(
        arguments(&plan)[4..],
        ["container", "start", "--", OF_THE_OWNER]
    );
    // A restart of a container that runs is permitted with no restart policy.
    world.inventory.containers[1].running = true;
    let plan = acted(OF_ADOPTED, Action::Restart, &world.facts()).expect("it runs");
    assert_eq!(
        arguments(&plan)[4..],
        ["container", "restart", "--", OF_ADOPTED]
    );
}

#[test]
fn k10_6_4_1_a_second_item_that_changes_docker_is_refused_but_a_stop_and_a_follow_are_not() {
    let mut world = World::new();
    world.busy = true;
    assert_eq!(
        acted(OF_ANIMA, Action::Start, &world.facts()),
        Err(Refusal::Busy)
    );
    assert_eq!(
        acted(OF_ANIMA, Action::Restart, &world.facts()),
        Err(Refusal::Busy)
    );
    assert!(acted(OF_ANIMA, Action::Stop, &world.facts()).is_ok());
    assert!(followed(OF_ANIMA, 0, &world.facts()).is_ok());
}

#[test]
fn k10_6_4_11_a_follow_reads_the_log_of_one_container_that_the_type_reaches() {
    let world = World::new();
    let plan = followed(OF_ANIMA, 200, &world.facts()).expect("reached");
    assert_eq!(
        arguments(&plan)[4..],
        [
            "container",
            "logs",
            "--follow",
            "--tail",
            "200",
            "--",
            OF_ANIMA
        ]
    );
    assert_eq!(
        followed(OF_THE_OWNER, 200, &world.facts()),
        Err(Refusal::NotAdopted)
    );
    assert_eq!(
        followed(&"d".repeat(64), 1, &world.facts()),
        Err(Refusal::NoContainer)
    );
}

#[test]
fn k10_6_4_1_a_command_is_a_program_with_arguments_and_only_the_environment_of_the_program() {
    let world = World::new();
    let plan = acted(OF_ANIMA, Action::Stop, &world.facts()).expect("reached");
    let command = commands(&plan)[0];
    assert_eq!(command.program, "/usr/bin/docker");
    assert_eq!(
        command.environment,
        [(
            "DOCKER_CONFIG".to_owned(),
            "/var/lib/anima/docker-configuration".to_owned()
        )]
    );
    assert_eq!(command.folder, None);
    // The socket path and the configuration folder are arguments, and no argument is a line
    // of a shell.
    assert!(
        command
            .arguments
            .iter()
            .all(|argument| !argument.contains(' '))
    );
}

#[test]
fn k10_6_4_3_an_identifier_of_the_engine_that_has_a_wrong_form_is_never_an_argument() {
    let mut world = World::new();
    world.inventory.containers[0].name = "--privileged".to_owned();
    assert_eq!(stop(&world, OF_ANIMA), Err(Refusal::NoContainer));
}
