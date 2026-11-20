//! What a problem of the check and a result of the converter may hold: never a value of the
//! text, which can be a secret of the operator (HOST-022, LOG-007).

use super::super::{ProblemCode, check, convert};

/// The value that no problem may hold.
const SECRET: &str = "hunter2";

#[test]
fn host_022_a_problem_never_holds_a_value_of_the_file() {
    let file = format!(
        r#"version: {SECRET}
name: {SECRET}
"{SECRET} is my password": 1
{SECRET}/top: 1
services:
  web:
    image: registry.example/{SECRET}:1
    image: {SECRET}
    restart: {SECRET}
    ports: {SECRET}
    environment: PASSWORD={SECRET}
    command: {{ {SECRET}: {SECRET} }}
    "PASSWORD={SECRET}": 1
    [{SECRET}]: 1
  "db {SECRET}": {SECRET}
  {SECRET}@db:
    environment:
      PASSWORD: {SECRET}
  worker: [{SECRET}]
"#
    );
    let found = check(&file);
    assert!(found.problems.len() >= 12, "{:?}", found.problems);
    assert!(!format!("{found:?}").contains(SECRET), "{found:?}");
    // Text that is not YAML: the position, and nothing of the line.
    let broken = check(&format!("services:\n  web:\n    image: \"{SECRET}\n"));
    assert_eq!(broken.problems.len(), 1);
    assert!(!format!("{broken:?}").contains(SECRET));
}

#[test]
fn host_022_a_problem_names_only_a_key_that_compose_defines_and_a_service_by_its_number() {
    // A secret of safe characters in the place of a key or of the name of a service.
    let token = "ghp_0123456789abcdefABCDEF0123456789abcd";
    let file = format!(
        "{SECRET}: 1\nx-{SECRET}: 1\nservices:\n  {token}:\n    {SECRET}: 1\n    ports: 80\n  \
         {token}:\n    image: a\n    image: b\n"
    );
    let found = check(&file);
    let names: Vec<_> = found
        .problems
        .iter()
        .map(|problem| (problem.code, problem.service_number, problem.key))
        .collect();
    assert_eq!(
        names,
        [
            (ProblemCode::UnknownTopLevelKey, None, None),
            (ProblemCode::NoImageOrBuild, Some(1), None),
            (ProblemCode::UnknownServiceKey, Some(1), None),
            (ProblemCode::WrongType, Some(1), Some("ports")),
            (ProblemCode::DuplicateKey, Some(2), None),
            (ProblemCode::DuplicateKey, Some(2), Some("image")),
        ]
    );
    let shown = format!("{found:?}");
    assert!(!shown.contains(SECRET) && !shown.contains(token), "{shown}");
}

#[test]
fn host_022_an_option_that_is_not_converted_is_listed_by_its_name_only() {
    let command = format!(
        "docker run --{SECRET} --cidfile /tmp/{SECRET} --password={SECRET} --{SECRET}!x=1 --ip {SECRET} \
         --mount type=odd,source={SECRET} --pull {SECRET} --gpus {SECRET} \
         --log-opt {SECRET} --ulimit {SECRET} --network {SECRET}/x --stop-timeout {SECRET} \
         --health-retries {SECRET} --init={SECRET} nginx"
    );
    let done = convert(&command).expect("converted");
    assert_eq!(
        done.not_converted,
        [
            "(an unknown option)",
            "--cidfile",
            "--ip",
            "--mount",
            "--pull",
            "--gpus",
            "--log-opt",
            "--ulimit",
            "--network",
            "--stop-timeout",
            "--health-retries",
            "--init",
        ]
    );
    assert!(!done.compose_file.contains(SECRET), "{}", done.compose_file);
    // A refusal is a code and a position.
    let refused = convert(&format!(
        "docker run -e A={SECRET} $(cat /etc/{SECRET}) nginx"
    ));
    assert!(!format!("{refused:?}").contains(SECRET));
}
