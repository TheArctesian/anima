//! A server, or only its `postmaster.pid`, that an earlier copy left.

use std::fs;
use std::mem;
use std::path::Path;
use std::process::{Command, Stdio};

use super::pid_file_text;
use crate::scratch::{Database, LIMITS};
use crate::server::{ServerError, is_postgres_on, stop_left_over};

#[test]
fn a_server_left_by_a_killed_copy_is_stopped_before_the_next_start() {
    let database = Database::new();
    let left = database.start();
    let left_process = left.process_id();
    // The copy that owned the server is gone and nothing stops its server.
    mem::forget(left);

    assert!(stop_left_over(&database.lock, &database.tree, &database.layout, LIMITS).unwrap());

    assert!(!database.pid_file().exists());
    let server = database.start();
    assert_ne!(server.process_id(), left_process);
    assert_eq!(database.sql("select 1").unwrap(), "1");
}

#[test]
fn a_stale_file_that_names_another_program_gets_no_signal_and_blocks_no_start() {
    let database = Database::new();
    // A live process of this user that is not PostgreSQL, with the number a dead server left.
    let mut bystander = Command::new("sleep")
        .arg("600")
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    fs::write(
        database.pid_file(),
        pid_file_text(bystander.id(), 1, "ready"),
    )
    .unwrap();

    let was_there =
        stop_left_over(&database.lock, &database.tree, &database.layout, LIMITS).unwrap();

    assert!(!was_there);
    assert!(
        bystander.try_wait().unwrap().is_none(),
        "the bystander was signalled"
    );
    let _server = database.start();
    assert_eq!(database.sql("select 1").unwrap(), "1");
    bystander.kill().unwrap();
    bystander.wait().unwrap();
}

#[test]
fn a_pid_file_that_cannot_be_read_is_an_error_not_an_absent_server() {
    let database = Database::new();
    fs::create_dir(database.pid_file()).unwrap();

    let outcome = stop_left_over(&database.lock, &database.tree, &database.layout, LIMITS);

    assert!(
        matches!(outcome, Err(ServerError::Io { .. })),
        "{outcome:?}"
    );
}

#[test]
fn only_a_postgres_started_on_this_cluster_counts_as_its_server() {
    let cluster = Path::new("/Users/a/Application Support/db/cluster");

    assert!(is_postgres_on(
        "/Applications/Anima.app/x/bin/postgres -D /Users/a/Application Support/db/cluster -c port=5432",
        cluster
    ));
    assert!(is_postgres_on(
        "postgres -D /Users/a/Application Support/db/cluster",
        cluster
    ));
    assert!(
        !is_postgres_on(
            "/opt/homebrew/bin/postgres -D /opt/homebrew/var/postgresql@17",
            cluster
        ),
        "another cluster's server"
    );
    assert!(
        !is_postgres_on("sleep -D /Users/a/Application Support/db/cluster", cluster),
        "another program"
    );
    assert!(!is_postgres_on("", cluster));
    assert!(
        !is_postgres_on(
            "postgres -D /Users/a/Application Support/db/cluster-copy -c port=5432",
            cluster
        ),
        "a cluster whose path only starts with this one"
    );
    assert!(
        is_postgres_on(
            "/Users/a/My -Apps/bin/postgres -D /Users/a/Application Support/db/cluster",
            cluster
        ),
        "a tree path with a space and a dash"
    );
}
