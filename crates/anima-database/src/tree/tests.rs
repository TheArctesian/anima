use std::fs;

use std::path::Path;

use super::{Tree, TreeError, major_in, sealed_command};
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
fn log_007_a_program_sees_none_of_the_callers_environment() {
    // The test process has at least PATH; `env` prints what a child really gets.
    assert!(std::env::var_os("PATH").is_some());

    let output = sealed_command(Path::new("/usr/bin/env")).output().unwrap();

    let mut seen: Vec<&str> = std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .collect();
    seen.sort_unstable();
    assert_eq!(
        seen,
        [
            "LC_ALL=C",
            "PGPASSFILE=/dev/null",
            "PGSERVICEFILE=/dev/null",
            "PGSYSCONFDIR=/dev/null",
            "TZ=UTC",
        ]
    );
}
