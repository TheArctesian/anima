//! What a problem of the check and a result of the converter may hold: never a value of the
//! text, which can be a secret of the operator (HOST-022, LOG-007).

use super::super::{check, convert};

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
fn host_022_a_name_in_a_problem_is_safe_characters_and_at_most_64() {
    let long = "a".repeat(200);
    let file = format!("services:\n  {long}:\n    {long}: 1\n  \"two words\":\n    odd key!: 1\n");
    let found = check(&file);
    let names: Vec<_> = found
        .problems
        .iter()
        .map(|problem| (problem.service.as_deref(), problem.key.as_deref()))
        .collect();
    let cut = "a".repeat(64);
    assert_eq!(
        names,
        [
            (Some(cut.as_str()), None),
            (Some(cut.as_str()), Some(cut.as_str())),
            (None, None),
            (None, None),
        ]
    );
}

#[test]
fn host_022_an_option_that_is_not_converted_is_listed_by_its_name_only() {
    let command = format!(
        "docker run --cidfile /tmp/{SECRET} --password={SECRET} --{SECRET}!x=1 --ip {SECRET} \
         --mount type=odd,source={SECRET} --pull {SECRET} --gpus {SECRET} \
         --log-opt {SECRET} --ulimit {SECRET} --network {SECRET}/x --stop-timeout {SECRET} \
         --health-retries {SECRET} --init={SECRET} nginx"
    );
    let done = convert(&command).expect("converted");
    assert_eq!(
        done.not_converted,
        [
            "--cidfile",
            "--password",
            "(an option)",
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
