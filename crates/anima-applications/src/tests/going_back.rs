//! `application_back` (K10 Section 6.4.9).

use compose_rules::Rule;

use super::{
    APPLICATION, FILES, PROJECT, World,
    applying::{COMPOSE, ENVIRONMENT, digests},
    commands,
};
use crate::{
    Refusal, Step,
    work::back::{Kept, accepted, prepared},
};

const KEPT: Kept<'static> = Kept {
    compose: COMPOSE.as_bytes(),
    environment: ENVIRONMENT.as_bytes(),
};

fn with_a_kept_version() -> World {
    let mut world = World::new();
    world.records.applications[0].kept = Some(digests());
    world
}

#[test]
fn k10_6_4_9_an_application_with_no_kept_version_is_refused() {
    let world = World::new();
    assert_eq!(
        accepted(APPLICATION, &world.facts()),
        Err(Refusal::NothingKept)
    );
    assert_eq!(
        prepared(APPLICATION, &KEPT, &world.facts()).err(),
        Some(Refusal::NothingKept)
    );
    let unknown = "0192f6b0-7c1e-7a3b-9d2f-000000000000";
    assert_eq!(accepted(unknown, &world.facts()), Err(Refusal::NothingKept));
    assert_eq!(
        accepted(APPLICATION, &with_a_kept_version().facts()),
        Ok(())
    );
}

#[test]
fn k10_6_4_9_the_kept_version_starts_with_the_kept_digests_and_no_pull() {
    let world = with_a_kept_version();
    let planned = prepared(APPLICATION, &KEPT, &world.facts()).expect("a kept version");
    let commands = commands(&planned.plan);
    assert_eq!(commands.len(), 1, "no pull and no tag: one command");
    let verbs = commands[0].arguments[10..].join(" ");
    assert_eq!(verbs, "up -d --remove-orphans --pull never --no-build");
    assert_eq!(commands[0].arguments[5], PROJECT);
    // It does not bring back data, and the result says so.
    assert!(planned.holds_volumes);
}

#[test]
fn k10_6_4_9_a_second_back_is_refused_until_the_next_apply() {
    let world = with_a_kept_version();
    let planned = prepared(APPLICATION, &KEPT, &world.facts()).expect("a kept version");
    let steps = &planned.plan.steps;
    let start = steps
        .iter()
        .position(|step| matches!(step, Step::Run(_)))
        .expect("a start");
    let deleted: Vec<&str> = steps[start..]
        .iter()
        .filter_map(|step| match step {
            Step::Delete { path } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    // After the start the plan deletes the kept version: the next item has nothing kept.
    assert_eq!(deleted.len(), 3);
    assert!(
        deleted
            .iter()
            .all(|path| path.starts_with(&format!("{FILES}/kept-")))
    );
}

#[test]
fn k10_6_4_9_the_kept_files_pass_the_rules_of_today() {
    let world = with_a_kept_version();
    let old = COMPOSE.replacen("    tmpfs: /run", "    privileged: true", 1);
    let kept = Kept {
        compose: old.as_bytes(),
        ..KEPT
    };
    assert_eq!(
        prepared(APPLICATION, &kept, &world.facts()).err(),
        Some(Refusal::FileNotPermitted {
            rule: Rule::KeyNotPermitted,
            service: Some(0)
        })
    );
    let mut world = with_a_kept_version();
    world.records.applications[0].kept = Some(digests()[..1].to_vec());
    assert_eq!(
        prepared(APPLICATION, &KEPT, &world.facts()).err(),
        Some(Refusal::NoDigest)
    );
}

#[test]
fn k10_6_4_9_a_back_has_the_refusals_of_an_apply_for_its_project() {
    let mut world = with_a_kept_version();
    world.busy = true;
    assert_eq!(accepted(APPLICATION, &world.facts()), Err(Refusal::Busy));
    world.busy = false;
    world.settings.protected = vec![PROJECT.to_owned()];
    assert_eq!(
        accepted(APPLICATION, &world.facts()),
        Err(Refusal::Protected)
    );
}
