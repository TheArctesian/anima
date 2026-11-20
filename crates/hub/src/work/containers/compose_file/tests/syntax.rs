//! Good files, and each problem with its code, its position and its names.

use super::super::{Place, ProblemCode, Severity, check};
use super::codes;

const DAWARICH: &str = r#"# The usual shape of a project.
name: dawarich
x-common: &common
  restart: unless-stopped
  logging:
    driver: json-file
    options: { max-size: "10m" }
services:
  app:
    <<: *common
    image: freikin/dawarich:latest
    container_name: dawarich_app
    command: ["bin/rails", "server", "-p", "3000"]
    entrypoint: web-entrypoint.sh
    ports:
      - "3000:3000"
      - target: 9394
        published: 9394
    environment:
      RAILS_ENV: production
      DATABASE_PASSWORD: ${DATABASE_PASSWORD:?set it}
    env_file: .env
    volumes:
      - public:/var/app/public
      - type: bind
        source: ./config
        target: /config
        read_only: true
    depends_on:
      db:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "curl -f localhost:3000 || exit 1"]
      interval: 10s
    labels:
      - "traefik.enable=true"
    networks: [default, backend]
    x-note: an extension of a service
  db:
    <<: *common
    image: postgis/postgis:17-3.5-alpine
    restart: "on-failure:5"
    environment:
      - POSTGRES_PASSWORD
    depends_on: []
    shm_size: 1g
  worker:
    extends:
      service: app
    command: sidekiq
  built:
    build: ./worker
    restart: "no"
networks:
  backend:
    external: true
volumes:
  public:
configs: {}
secrets: {}
"#;

#[test]
fn host_022_a_file_as_people_write_it_has_no_problem() {
    let found = check(DAWARICH);
    assert_eq!(found.problems, []);
    assert!(found.syntax_valid() && !found.cut);
    for good in [
        "services:\n  web:\n    image: nginx\n",
        "services: { web: { image: nginx, ports: ['80:80'] } }",
        "---\nservices:\n  web:\n    build: .\n...\n",
        "include:\n  - other.yaml\n",
        "services:\n  web: &web\n    image: nginx\n  two: *web\n",
        "services:\n  web:\n    image: nginx\n    restart: ${RESTART:-always}\n    ports:\n",
    ] {
        assert_eq!(codes(good), [], "{good}");
    }
}

/// A file with one problem, and the problem: code, line, column, number of the service, key.
type Case = (
    &'static str,
    ProblemCode,
    (u32, u32),
    Option<u32>,
    Option<&'static str>,
);

const ERRORS: [Case; 19] = [
    ("services: [", ProblemCode::NotYaml, (2, 1), None, None),
    (
        "services:\n  web:\n    image: a\n---\nservices: {}\n",
        ProblemCode::MoreThanOneDocument,
        (4, 1),
        None,
        None,
    ),
    ("", ProblemCode::TopLevelNotMapping, (1, 1), None, None),
    (
        "# only a comment\n",
        ProblemCode::TopLevelNotMapping,
        (1, 1),
        None,
        None,
    ),
    (
        "- services\n",
        ProblemCode::TopLevelNotMapping,
        (1, 1),
        None,
        None,
    ),
    (
        "just text",
        ProblemCode::TopLevelNotMapping,
        (1, 1),
        None,
        None,
    ),
    (
        "services:\n  web:\n    image: a\n    [a, b]: c\n",
        ProblemCode::KeyNotText,
        (4, 5),
        Some(1),
        None,
    ),
    (
        "services:\n  web:\n    image: a\n    image: b\n",
        ProblemCode::DuplicateKey,
        (4, 5),
        Some(1),
        Some("image"),
    ),
    (
        "services:\n  web:\n    image: a\n  web:\n    image: b\n",
        ProblemCode::DuplicateKey,
        (4, 3),
        Some(2),
        None,
    ),
    ("name: app\n", ProblemCode::NoServices, (1, 1), None, None),
    (
        "services: web\n",
        ProblemCode::ServicesNotMapping,
        (1, 1),
        None,
        None,
    ),
    (
        "services:\n  - web\n",
        ProblemCode::ServicesNotMapping,
        (1, 1),
        None,
        None,
    ),
    (
        "services:\n",
        ProblemCode::ServicesEmpty,
        (1, 1),
        None,
        None,
    ),
    (
        "services: {}\n",
        ProblemCode::ServicesEmpty,
        (1, 1),
        None,
        None,
    ),
    (
        "services:\n  web: nginx\n",
        ProblemCode::ServiceNotMapping,
        (2, 3),
        Some(1),
        None,
    ),
    (
        "services:\n  web:\n    ports: ['80:80']\n",
        ProblemCode::NoImageOrBuild,
        (2, 3),
        Some(1),
        None,
    ),
    (
        "services:\n  web:\n    image: a\n    ports: 80\n",
        ProblemCode::WrongType,
        (4, 5),
        Some(1),
        Some("ports"),
    ),
    (
        "services:\n  web:\n    image: a\n    restart: sometimes\n",
        ProblemCode::RestartNotKnown,
        (4, 5),
        Some(1),
        Some("restart"),
    ),
    (
        "services:\n  web:\n    image: a\n    restart: on-failure:x\n",
        ProblemCode::RestartNotKnown,
        (4, 5),
        Some(1),
        Some("restart"),
    ),
];

#[test]
fn host_022_each_error_has_its_code_its_position_and_its_names() {
    for (file, code, (line, column), service, key) in ERRORS {
        let found = check(file);
        let [problem] = found.problems.as_slice() else {
            panic!("{file}: {:?}", found.problems);
        };
        assert_eq!(problem.code, code, "{file}");
        assert_eq!(problem.place, Place { line, column }, "{file}");
        assert_eq!(problem.service_number, service, "{file}");
        assert_eq!(problem.key, key, "{file}");
        assert_eq!(code.severity(), Severity::Error);
        assert!(!found.syntax_valid(), "{file}");
    }
}

#[test]
fn host_022_a_wrong_form_of_each_common_key_is_an_error() {
    for wrong in [
        "ports: '80:80'",
        "ports: { a: b }",
        "volumes: data:/data",
        "environment: A=b",
        "depends_on: db",
        "command: { a: b }",
        "labels: a=b",
        "healthcheck: [a]",
        "image: [nginx]",
        "build: [a]",
    ] {
        let file = format!("services:\n  web:\n    image: a\n    build: .\n    {wrong}\n");
        let expected = if wrong.starts_with("image") || wrong.starts_with("build") {
            vec![ProblemCode::DuplicateKey, ProblemCode::WrongType]
        } else {
            vec![ProblemCode::WrongType]
        };
        assert_eq!(codes(&file), expected, "{wrong}");
    }
    for right in [
        "ports: ['80:80']",
        "environment: { A: b }",
        "environment: [A=b]",
        "depends_on: [db]",
        "depends_on: { db: { condition: service_started } }",
        "command: nginx -g 'daemon off;'",
        "command: [nginx]",
        "command:",
        "labels: { a: b }",
        "restart: always",
        "user: 1000",
        "privileged: true",
    ] {
        let file = format!("services:\n  web:\n    image: a\n    {right}\n");
        assert_eq!(codes(&file), [], "{right}");
    }
}

#[test]
fn host_022_a_key_that_compose_does_not_know_and_version_are_warnings() {
    let file = "version: '3.8'\nservics: {}\nx-mine: 1\nservices:\n  web:\n    image: a\n    \
                imge: b\n    x-mine: 1\n";
    let found = check(file);
    let seen: Vec<_> = found
        .problems
        .iter()
        .map(|problem| {
            let place = (problem.place.line, problem.place.column);
            (problem.code, place, problem.key)
        })
        .collect();
    assert_eq!(
        seen,
        [
            (ProblemCode::VersionObsolete, (1, 1), None),
            (ProblemCode::UnknownTopLevelKey, (2, 1), None),
            (ProblemCode::UnknownServiceKey, (7, 5), None),
        ]
    );
    assert_eq!(found.problems[2].service_number, Some(1));
    assert!(
        found
            .problems
            .iter()
            .all(|p| p.code.severity() == Severity::Warning)
    );
    assert!(found.syntax_valid());
}

#[test]
fn host_022_the_check_follows_no_merge_and_no_alias() {
    // The image can come from the anchor, so a service with a merge key is not refused.
    let merged = "x-base: &base\n  image: a\nservices:\n  web:\n    <<: *base\n    <<: *base\n";
    assert_eq!(codes(merged), []);
    let aliased = "x-ports: &ports '80:80'\nservices:\n  web:\n    image: a\n    ports: *ports\n";
    assert_eq!(codes(aliased), []);
}
