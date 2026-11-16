use std::fs;

use super::super::{
    LocalHubError, carried,
    update::{Updated, parse_updated, update, update_args},
};
use super::programs::{RELEASE, bundle};

#[test]
fn the_answer_says_whether_the_programs_were_replaced() {
    assert_eq!(parse_updated(br#"{"updated":true}"#), Ok(Updated::Replaced));
    assert_eq!(
        parse_updated(b"{\"updated\":false}\n"),
        Ok(Updated::Unchanged)
    );
    assert_eq!(
        parse_updated(br#"{"updated":false,"older":true}"#),
        Ok(Updated::KeptNewer)
    );
}

#[test]
fn an_answer_that_is_not_the_programs_is_a_failure() {
    for printed in [&b""[..], b"installed: /x", br#"{"updated":"yes"}"#, b"{}"] {
        assert_eq!(parse_updated(printed), Err(LocalHubError::Failed));
    }
}

#[test]
fn the_carried_program_is_asked_to_update_from_the_tree_of_the_bundle() {
    let folder = tempfile::tempdir().unwrap();
    let asked = folder.path().join("asked");
    let script = format!(
        "printf '%s\\n' \"$@\" > '{}'\necho '{{\"updated\":true}}'",
        asked.display()
    );
    let executable = bundle(folder.path(), cfg!(target_os = "macos"), &script);
    let found = carried::beside(&executable, cfg!(target_os = "macos"), true, RELEASE).unwrap();

    assert_eq!(update(&found), Ok(Updated::Replaced));

    let args = update_args(&found.tree).map(|arg| arg.into_string().unwrap());
    assert_eq!(fs::read_to_string(asked).unwrap(), args.join("\n") + "\n");
}

#[test]
fn a_program_that_fails_updates_nothing_and_says_so() {
    let folder = tempfile::tempdir().unwrap();
    let executable = bundle(folder.path(), cfg!(target_os = "macos"), "exit 1");
    let found = carried::beside(&executable, cfg!(target_os = "macos"), true, RELEASE).unwrap();

    assert_eq!(update(&found), Err(LocalHubError::Failed));
}
