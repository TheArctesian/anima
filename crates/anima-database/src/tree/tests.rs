use std::fs;

use super::{Program, Tree, TreeError, major_in};
use crate::local_tree::local_tree;

#[test]
fn the_major_version_is_read_from_the_version_line() {
    assert_eq!(
        major_in("postgres (PostgreSQL) 17.12 (Homebrew)\n"),
        Some(17)
    );
    assert_eq!(major_in("postgres (PostgreSQL) 17.12\n"), Some(17));
    assert_eq!(major_in("postgres (PostgreSQL) 18beta1\n"), Some(18));
    assert_eq!(major_in("postgres (PostgreSQL) 9.6.24\n"), Some(9));
}

#[test]
fn a_line_with_no_version_gives_none() {
    assert_eq!(major_in(""), None);
    assert_eq!(major_in("postgres (PostgreSQL) devel"), None);
    assert_eq!(major_in("psql 17.12"), None);
}

#[test]
fn the_local_tree_is_postgresql_17() {
    assert_eq!(local_tree().major_version().unwrap(), 17);
}

#[test]
fn a_folder_without_the_server_programs_is_not_a_tree() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir(folder.path().join("bin")).unwrap();
    fs::write(folder.path().join("bin/postgres"), "").unwrap();

    assert!(matches!(
        Tree::at(folder.path()),
        Err(TreeError::ProgramMissing { program: "initdb" })
    ));
}

#[test]
fn a_command_carries_none_of_the_callers_environment() {
    let command = local_tree().command(Program::Postgres);

    let names: Vec<_> = command
        .get_envs()
        .filter_map(|(name, value)| value.map(|_| name.to_string_lossy().into_owned()))
        .collect();

    assert_eq!(names, ["LC_ALL", "TZ"]);
}
