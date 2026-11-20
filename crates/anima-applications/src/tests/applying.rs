//! `application_apply` (K10 Sections 6.4.4 to 6.4.6).

use compose_rules::Rule;

use super::{APPLICATION, DIGEST, FILES, FIRST, PROJECT, World, commands, pulled};
use crate::{
    Apply, Pulled, Refusal, Step,
    work::apply::{Files, Planned, Prepared, accepted, planned, prepared, sha256},
};

pub(super) const COMPOSE: &str = r#"
services:
  app:
    image: example/tracks:1
    command: ["bin/rails", "server"]
    environment:
      PRICE: "$$5 ${TOKEN}"
    mem_limit: 512m
    cpus: "0.5"
    stop_grace_period: 1m30s
    cap_drop: [ALL]
    tmpfs: /run
    healthcheck:
      test: ["CMD", "true"]
      interval: 1m
      retries: 3
    ports:
      - target: 3000
        published: 8080
        host_ip: 127.0.0.1
    volumes:
      - type: volume
        source: store
        target: /store
      - type: bind
        source: ./binds/config
        target: /config
        read_only: true
    configs:
      - source: settings
        target: /etc/tracks.yml
        mode: "0444"
  db:
    image: postgres:17
    depends_on:
      app:
    networks:
      inner:
        aliases: database
volumes:
  store:
networks:
  inner:
    internal: true
configs:
  settings:
    content: "a\u2028b"
"#;

pub(super) const ENVIRONMENT: &str = "TOKEN=s3cr$t\n";

pub(super) fn digests() -> Vec<Pulled> {
    vec![
        pulled("app", "example/tracks:1"),
        pulled("db", "postgres:17"),
    ]
}

fn apply(compose: &str, environment: Option<&str>) -> Apply {
    Apply {
        application: APPLICATION.to_owned(),
        file_sha256: sha256(compose.as_bytes()),
        environment_sha256: environment.map(|text| sha256(text.as_bytes())),
        pull: true,
        keep: false,
    }
}

fn prepare(world: &World, compose: &str) -> Result<Prepared, Refusal> {
    let files = Files {
        compose: compose.as_bytes(),
        environment: Some(ENVIRONMENT.as_bytes()),
    };
    prepared(&apply(compose, Some(ENVIRONMENT)), &files, &world.facts())
}

fn plan(world: &World, keep: bool) -> Planned {
    let files = Files {
        compose: COMPOSE.as_bytes(),
        environment: Some(ENVIRONMENT.as_bytes()),
    };
    let apply = Apply {
        keep,
        ..apply(COMPOSE, Some(ENVIRONMENT))
    };
    let prepared = prepared(&apply, &files, &world.facts()).expect("the files pass");
    planned(&prepared, &digests(), &world.facts()).expect("each service has a digest")
}

#[test]
fn k10_6_4_6_the_sha_256_is_the_one_of_the_standard() {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn k10_6_4_3_an_apply_of_a_protected_project_is_refused() {
    let mut world = World::new();
    let apply = apply(COMPOSE, None);
    assert_eq!(accepted(&apply, &world.facts()), Ok(()));
    world.settings.protected = vec![PROJECT.to_owned()];
    assert_eq!(accepted(&apply, &world.facts()), Err(Refusal::Protected));
    world.settings.protected = vec!["anima-app-1".to_owned()];
    assert_eq!(accepted(&apply, &world.facts()), Err(Refusal::Protected));
}

#[test]
fn k10_6_4_6_a_project_of_that_name_with_a_different_folder_is_refused() {
    let mut world = World::new();
    let apply = apply(COMPOSE, None);
    world.inventory.containers[0].folder = Some("/home/ada/project".to_owned());
    assert_eq!(accepted(&apply, &world.facts()), Err(Refusal::NameTaken));
    world.inventory.containers[0].folder = None;
    assert_eq!(accepted(&apply, &world.facts()), Err(Refusal::NameTaken));
    world.inventory.containers[0].folder = Some(FILES.to_owned());
    assert_eq!(accepted(&apply, &world.facts()), Ok(()));
}

#[test]
fn k10_6_4_1_an_apply_is_refused_while_an_item_that_changes_docker_runs() {
    let mut world = World::new();
    world.busy = true;
    assert_eq!(
        accepted(&apply(COMPOSE, None), &world.facts()),
        Err(Refusal::Busy)
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
        super::applying::prepared(&apply, &files, &world.facts()),
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
