//! The plan of an `application_apply` (K10 Sections 6.4.4 to 6.4.6).

use super::{COMPOSE, ENVIRONMENT, digests, plan, prepare};
use crate::{
    Refusal, Step,
    tests::{DIGEST, FILES, FIRST, PROJECT, World, commands},
    work::apply::{Planned, planned, sha256},
};

#[test]
fn k10_6_4_4_a_service_with_no_digest_of_its_pull_is_never_started() {
    let world = World::new();
    let prepared = prepare(&world, COMPOSE).expect("the files pass");
    let facts = world.facts();
    assert_eq!(
        planned(&prepared, &digests()[..1], &facts).err(),
        Some(Refusal::NoDigest)
    );
    let mut wrong = digests();
    wrong[1].digest = "sha256:latest".to_owned();
    assert_eq!(
        planned(&prepared, &wrong, &facts).err(),
        Some(Refusal::NoDigest)
    );
    // An image that the file names by a digest is started by that digest only.
    let other = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let by_digest = COMPOSE.replacen("postgres:17", &format!("postgres@{other}"), 1);
    let prepared = prepare(&world, &by_digest).expect("the files pass");
    let mut pulled = digests();
    pulled[1].image = format!("postgres@{other}");
    assert_eq!(
        planned(&prepared, &pulled, &facts).err(),
        Some(Refusal::NoDigest)
    );
    pulled[1].digest = other.to_owned();
    assert!(planned(&prepared, &pulled, &facts).is_ok());
}

#[test]
fn k10_6_4_5_the_plan_writes_the_files_below_files_and_makes_each_folder_of_binds() {
    let world = World::new();
    let planned = plan(&world, false);
    let application = "/var/lib/anima/applications/0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e";
    let folders: Vec<&str> = planned
        .plan
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Folder { path } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    let binds = format!("{application}/binds");
    assert_eq!(
        folders,
        [
            application,
            FILES,
            binds.as_str(),
            &format!("{binds}/config")
        ]
    );
    let written = |name: &str| {
        let path = format!("{FILES}/{name}");
        planned.plan.steps.iter().find_map(|step| match step {
            Step::Write { path: to, content } if *to == path => Some(content.clone()),
            _ => None,
        })
    };
    assert_eq!(
        written("compose-as-it-came.yaml").as_deref(),
        Some(COMPOSE.as_bytes())
    );
    assert_eq!(
        written("environment-of-the-application").as_deref(),
        Some(ENVIRONMENT.as_bytes())
    );
    let form = written("final-form.json").expect("the final form");
    assert_eq!(planned.final_form_sha256, sha256(&form));
    assert!(written("digests.json").is_some_and(|text| text.starts_with(b"[{\"service\":\"app\"")));
    let pulled =
        String::from_utf8(written("pulled.json").expect("the pulled images")).expect("text");
    assert_eq!(
        pulled,
        format!("[\"example/tracks@{DIGEST}\",\"postgres@{DIGEST}\"]\n")
    );
    // Each path of a step is below the folder of the application, and none is `.env`.
    for step in &planned.plan.steps {
        if let Step::Write { path, .. } | Step::Folder { path } = step {
            assert!(
                path.starts_with(application) && !path.ends_with("/.env"),
                "{path}"
            );
        }
    }
}

#[test]
fn k10_6_4_4_the_project_starts_from_the_final_form_only() {
    let world = World::new();
    let planned = plan(&world, false);
    let steps = &planned.plan.steps;
    let commands = commands(&planned.plan);
    assert_eq!(commands.len(), 1);
    let up = commands[0];
    assert_eq!(up.program, "/usr/libexec/docker/cli-plugins/docker-compose");
    assert_eq!(up.arguments[..4], FIRST);
    let final_form = format!("{FILES}/final-form.json");
    assert_eq!(
        up.arguments[4..],
        [
            "--project-name",
            PROJECT,
            "--project-directory",
            FILES,
            "--file",
            final_form.as_str(),
            "up",
            "-d",
            "--remove-orphans",
            "--pull",
            "never",
            "--no-build",
        ]
    );
    assert_eq!(up.folder.as_deref(), Some(FILES));
    // The environment file is never an argument, and no variable of it is in the environment.
    assert!(
        !up.arguments
            .iter()
            .any(|argument| argument.contains("environment"))
    );
    assert_eq!(up.environment.len(), 1);
    // Directly before the start: `files/` holds no `.env`, and each folder of `binds/` is real.
    let last = steps.len() - 1;
    assert!(matches!(&steps[last], Step::Run(_)));
    let binds = FILES.replace("/files", "/binds");
    assert_eq!(
        steps[last - 1],
        Step::RealFolder {
            path: format!("{binds}/config"),
            below: binds.clone()
        }
    );
    assert!(matches!(&steps[last - 2], Step::RealFolder { path, .. } if *path == binds));
    assert_eq!(
        steps[last - 3],
        Step::Absent {
            path: format!("{FILES}/.env")
        }
    );
}

#[test]
fn k10_6_4_6_keep_1_keeps_the_version_before_this_apply_and_keep_0_keeps_none() {
    let mut world = World::new();
    let moves = |planned: &Planned| {
        let moved = |step: &Step| matches!(step, Step::Move { from, to } if from.starts_with(FILES) && to.contains("/files/kept-"));
        planned.plan.steps.iter().filter(|step| moved(step)).count()
    };
    let deletes = |planned: &Planned| {
        let deleted =
            |step: &Step| matches!(step, Step::Delete { path } if path.contains("/files/kept-"));
        planned
            .plan
            .steps
            .iter()
            .filter(|step| deleted(step))
            .count()
    };
    let kept = plan(&world, true);
    assert_eq!((moves(&kept), deletes(&kept)), (3, 0));
    // The moves are before the first write of the new version.
    let first_move = kept
        .plan
        .steps
        .iter()
        .position(|step| matches!(step, Step::Move { .. }));
    let first_write = kept
        .plan
        .steps
        .iter()
        .position(|step| matches!(step, Step::Write { .. }));
    assert!(first_move < first_write);
    let none = plan(&world, false);
    assert_eq!((moves(&none), deletes(&none)), (0, 3));
    // The first install has no version to keep.
    world.records.applications.clear();
    world.inventory.containers.remove(0);
    let first = plan(&world, true);
    assert_eq!((moves(&first), deletes(&first)), (0, 3));
}
