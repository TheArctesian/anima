//! `application_remove` (K10 Section 6.4.8).

use super::{APPLICATION, DIGEST, FILES, FIRST, OF_ANIMA, PROJECT, World, commands, pulled};
use crate::{Installed, Network, Plan, Refusal, Removal, Step, Volume, work::removal::removed};

fn removal(volumes: bool, images: bool) -> Removal {
    Removal {
        application: APPLICATION.to_owned(),
        volumes,
        images,
    }
}

/// The arguments of each command of a plan, after the first four.
fn verbs(plan: &Plan) -> Vec<String> {
    commands(plan)
        .iter()
        .map(|command| {
            assert_eq!(command.arguments[..4], FIRST);
            command.arguments[4..].join(" ")
        })
        .collect()
}

/// The arguments of each command whose failure does not stop the plan, after the first four.
fn attempts(plan: &Plan) -> Vec<String> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            Step::Attempt(command) => Some(command.arguments[4..].join(" ")),
            _ => None,
        })
        .collect()
}

fn deleted(plan: &Plan) -> Vec<&str> {
    plan.steps
        .iter()
        .filter_map(|step| match step {
            Step::DeleteFolder { path } | Step::DeleteFolderInEachCase { path } => {
                Some(path.as_str())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn k10_6_4_8_a_removal_stops_the_project_from_the_kept_final_form_and_deletes_files() {
    let world = World::new();
    let plan = removed(&removal(false, false), &world.facts()).expect("installed");
    let down = format!(
        "--project-name {PROJECT} --project-directory {FILES} --file {FILES}/final-form.json down"
    );
    assert_eq!(verbs(&plan), [down]);
    // `files/` is deleted in each case; `binds/` stays.
    assert_eq!(deleted(&plan), [FILES]);
}

#[test]
fn k10_6_4_8_volumes_true_needs_the_hub_may_delete_volumes() {
    let mut world = World::new();
    assert_eq!(
        removed(&removal(true, false), &world.facts()),
        Err(Refusal::NotAtMachine)
    );
    world.settings.hub_may_delete_volumes = true;
    let plan = removed(&removal(true, false), &world.facts()).expect("the setting is on");
    assert!(verbs(&plan)[0].ends_with(" down --volumes"));
    let application = FILES.trim_end_matches("/files");
    assert_eq!(deleted(&plan), [FILES, application]);
}

#[test]
fn k10_6_4_8_images_true_removes_the_images_that_the_program_pulled_and_never_by_force() {
    let world = World::new();
    let plan = removed(&removal(false, true), &world.facts()).expect("installed");
    let attempts = attempts(&plan);
    assert_eq!(attempts, [format!("image rm -- example/tracks@{DIGEST}")]);
    assert!(!attempts.iter().any(|verb| verb.contains("--force")));
}

#[test]
fn k10_6_4_8_files_is_deleted_directly_after_the_stop_and_also_when_a_step_fails() {
    let mut world = World::new();
    world.settings.hub_may_delete_volumes = true;
    let plan = removed(&removal(true, true), &world.facts()).expect("installed");
    let kinds: Vec<&str> = plan
        .steps
        .iter()
        .map(|step| match step {
            Step::Absent { .. } => "absent",
            Step::Run(_) => "down",
            Step::DeleteFolderInEachCase { path } if path == FILES => "files in each case",
            Step::Attempt(_) => "image",
            Step::DeleteFolder { .. } => "application",
            _ => "other",
        })
        .collect();
    // A failed `down` or a refused image removal never leaves `files/`, which holds the
    // environment file and the final form. `binds/` goes only after a `down` that did not fail.
    assert_eq!(
        kinds,
        [
            "absent",
            "down",
            "files in each case",
            "image",
            "application"
        ]
    );
}

#[test]
fn k10_6_4_8_images_true_leaves_out_an_image_that_a_different_application_holds() {
    let mut world = World::new();
    world.records.applications.push(Installed {
        application: "0192f6b0-0000-7000-8000-000000000009".to_owned(),
        has_files: true,
        current: Vec::new(),
        kept: Some(vec![pulled("web", "docker.io/example/tracks:1")]),
        pulled: Vec::new(),
    });
    let plan = removed(&removal(false, true), &world.facts()).expect("installed");
    assert_eq!(attempts(&plan), Vec::<String>::new());
    world.records.applications[1].kept = None;
    let plan = removed(&removal(false, true), &world.facts()).expect("installed");
    assert_eq!(attempts(&plan).len(), 1);
}

#[test]
fn k10_6_4_8_when_the_folder_is_gone_the_removal_is_by_the_project_label() {
    let mut world = World::new();
    world.records.applications.clear();
    world.inventory.networks = vec![
        Network {
            name: "anima-net".to_owned(),
            project: Some(PROJECT.to_owned()),
            used: true,
            of_the_engine: false,
        },
        Network {
            name: "tracks_default".to_owned(),
            project: Some("tracks".to_owned()),
            used: true,
            of_the_engine: false,
        },
    ];
    world.inventory.volumes = vec![Volume {
        name: "anima-store".to_owned(),
        project: Some(PROJECT.to_owned()),
    }];
    let plan = removed(&removal(false, false), &world.facts()).expect("by the label");
    assert_eq!(
        verbs(&plan),
        [
            format!("container rm --force -- {OF_ANIMA}"),
            "network rm -- anima-net".to_owned()
        ]
    );
    world.settings.hub_may_delete_volumes = true;
    let plan = removed(&removal(true, false), &world.facts()).expect("by the label");
    assert_eq!(verbs(&plan)[2], "volume rm -- anima-store");
}

#[test]
fn k10_6_4_3_a_removal_of_an_application_that_anima_did_not_install_is_refused() {
    let mut world = World::new();
    world.records.applications.clear();
    world.inventory.containers.remove(0);
    assert_eq!(
        removed(&removal(false, false), &world.facts()),
        Err(Refusal::NotAdopted)
    );
}

#[test]
fn k10_6_4_3_a_removal_of_a_protected_project_is_refused_and_a_busy_machine_refuses() {
    let mut world = World::new();
    world.settings.protected = vec!["anima-app-1".to_owned()];
    assert_eq!(
        removed(&removal(false, false), &world.facts()),
        Err(Refusal::Protected)
    );
    world.settings.protected.clear();
    world.busy = true;
    assert_eq!(
        removed(&removal(false, false), &world.facts()),
        Err(Refusal::Busy)
    );
}
