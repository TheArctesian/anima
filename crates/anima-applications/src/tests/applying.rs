//! `application_apply` (K10 Sections 6.4.4 to 6.4.6): the check before the files here, the
//! check of the two files in `files`, and the plan in `planning`.

mod debug_text;
mod files;
mod planning;

use super::{APPLICATION, FILES, PROJECT, World, pulled};
use crate::{
    Apply, Pulled, Refusal,
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

pub(super) fn apply(compose: &str, environment: Option<&str>) -> Apply {
    Apply {
        application: APPLICATION.to_owned(),
        file_sha256: sha256(compose.as_bytes()),
        environment_sha256: environment.map(|text| sha256(text.as_bytes())),
        pull: true,
        keep: false,
    }
}

pub(super) fn prepare(world: &World, compose: &str) -> Result<Prepared, Refusal> {
    let files = Files {
        compose: compose.as_bytes(),
        environment: Some(ENVIRONMENT.as_bytes()),
    };
    prepared(&apply(compose, Some(ENVIRONMENT)), &files, &world.facts())
}

pub(super) fn plan(world: &World, keep: bool) -> Planned {
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
