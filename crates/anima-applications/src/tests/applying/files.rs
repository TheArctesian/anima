//! The check of the two files of an `application_apply` (K10 Sections 6.4.4 and 6.4.6).

use compose_rules::Rule;

use super::{COMPOSE, ENVIRONMENT, apply, digests, prepare};
use crate::{
    Apply, Refusal,
    tests::{FIRST, World},
    work::apply::{Files, planned, prepared, sha256},
};

#[test]
fn k10_6_4_6_the_sha_256_is_the_one_of_the_standard() {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn k10_6_4_6_a_file_whose_sha_256_is_different_is_refused() {
    let world = World::new();
    let sound = Files {
        compose: COMPOSE.as_bytes(),
        environment: Some(ENVIRONMENT.as_bytes()),
    };
    let named = apply(COMPOSE, Some(ENVIRONMENT));
    assert!(prepared(&named, &sound, &world.facts()).is_ok());
    let changed = format!("{COMPOSE}\n");
    for files in [
        Files {
            compose: changed.as_bytes(),
            ..sound
        },
        Files {
            environment: Some(b"TOKEN=other\n"),
            ..sound
        },
        // The item names an environment file and the daemon gives none.
        Files {
            environment: None,
            ..sound
        },
    ] {
        assert_eq!(
            prepared(&named, &files, &world.facts()),
            Err(Refusal::ProgramMismatch)
        );
    }
    // The daemon gives an environment file that the item does not name.
    let unnamed = apply(COMPOSE, None);
    assert_eq!(
        prepared(&unnamed, &sound, &world.facts()),
        Err(Refusal::ProgramMismatch)
    );
}

#[test]
fn k10_6_4_4_a_file_that_breaks_a_rule_is_refused_with_the_code_and_the_position() {
    let world = World::new();
    let privileged = COMPOSE.replacen(
        "    image: postgres:17",
        "    image: postgres:17\n    privileged: true",
        1,
    );
    let refusal = prepare(&world, &privileged).expect_err("refused");
    assert_eq!(
        refusal,
        Refusal::FileNotPermitted {
            rule: Rule::KeyNotPermitted,
            service: Some(1)
        }
    );
    // No refusal holds text of a file or of a value.
    let secret = COMPOSE.replacen("target: /store", "target: /hunter2/../etc", 1);
    let refusal = prepare(&world, &secret).expect_err("refused");
    assert_eq!(refusal.code(), "file_not_permitted");
    assert!(!format!("{refusal} {refusal:?}").contains("hunter2"));
    let not_text = Files {
        compose: &[0xff, 0xfe],
        environment: None,
    };
    let named = Apply {
        file_sha256: sha256(&[0xff, 0xfe]),
        ..apply(COMPOSE, None)
    };
    assert_eq!(
        prepared(&named, &not_text, &world.facts()),
        Err(Refusal::file(Rule::YamlNotPermitted))
    );
}

#[test]
fn k10_6_4_4_a_port_on_each_address_needs_open_to_the_network() {
    let mut world = World::new();
    let open = COMPOSE.replacen("host_ip: 127.0.0.1", "host_ip: 0.0.0.0", 1);
    assert_eq!(prepare(&world, &open).err(), Some(Refusal::NotAtMachine));
    world.settings.open_to_network = true;
    assert!(prepare(&world, &open).is_ok());
}

#[test]
fn k10_6_4_4_a_pull_is_refused_when_the_free_disk_is_below_the_limit() {
    let mut world = World::new();
    world.machine.free_disk_bytes = (10 << 30) - 1;
    assert_eq!(prepare(&world, COMPOSE).err(), Some(Refusal::LimitDisk));
    world.machine.free_disk_bytes = 10 << 30;
    assert!(prepare(&world, COMPOSE).is_ok());
}

#[test]
fn k10_6_4_6_pull_false_uses_the_digests_of_the_last_apply_and_is_refused_with_none() {
    let mut world = World::new();
    // No pull needs no free disk.
    world.machine.free_disk_bytes = 0;
    let files = Files {
        compose: COMPOSE.as_bytes(),
        environment: Some(ENVIRONMENT.as_bytes()),
    };
    let apply = Apply {
        pull: false,
        ..apply(COMPOSE, Some(ENVIRONMENT))
    };
    // The last apply has a digest of `app` only.
    assert_eq!(
        prepared(&apply, &files, &world.facts()),
        Err(Refusal::NoDigest)
    );
    world.records.applications[0].current = digests();
    let prepared = prepared(&apply, &files, &world.facts()).expect("each service has a digest");
    assert_eq!(prepared.pulls, Vec::new());
    assert!(planned(&prepared, &[], &world.facts()).is_ok());
    // A digest of the same service with a different image is not a digest of this image.
    world.records.applications[0].current[1].image = "postgres:16".to_owned();
    assert_eq!(
        crate::work::apply::prepared(&apply, &files, &world.facts()),
        Err(Refusal::NoDigest)
    );
}

#[test]
fn k10_6_4_4_an_image_is_pulled_as_an_operand_after_two_hyphens() {
    let world = World::new();
    let prepared = prepare(&world, COMPOSE).expect("the files pass");
    assert_eq!(prepared.pulls.len(), 2);
    let pull = &prepared.pulls[0];
    assert_eq!(
        (pull.service.as_str(), pull.image.as_str()),
        ("app", "example/tracks:1")
    );
    assert_eq!(pull.pull.program, "/usr/bin/docker");
    assert_eq!(pull.pull.arguments[..4], FIRST);
    assert_eq!(
        pull.pull.arguments[4..],
        ["image", "pull", "--", "example/tracks:1"]
    );
    assert_eq!(
        pull.inspect.arguments[4..],
        [
            "image",
            "inspect",
            "--format",
            "{{json .RepoDigests}}",
            "--",
            "example/tracks:1"
        ]
    );
}
