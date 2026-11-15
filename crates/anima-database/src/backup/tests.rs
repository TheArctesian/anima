use std::fs;
use std::io::{Seek as _, SeekFrom, Write as _};
use std::os::unix::fs::PermissionsExt;

use super::{BackupError, Checked, check, damaged_in, restore, write};
use crate::roles;
use crate::scratch::Database;

/// A database with the roles and some of the owner's rows.
fn database_with_rows() -> (Database, crate::Server) {
    let database = Database::new();
    let server = database.start();
    roles::prepare(&database.layout).unwrap();
    database
        .sql_in(
            "anima",
            "set role anima_owner; create schema devices; \
             create table devices.devices (id int primary key, name text not null); \
             insert into devices.devices select n, 'device ' || n from generate_series(1, 500) n; \
             grant usage on schema devices to anima_runtime; \
             grant select on devices.devices to anima_runtime",
        )
        .unwrap();
    (database, server)
}

const ROWS: &str = "select count(*) || ' ' || sum(id) || ' ' || \
     (select tableowner from pg_tables where tablename = 'devices') || ' ' || \
     has_table_privilege('anima_hub', 'devices.devices', 'select') from devices.devices";

#[test]
fn db_007_a_backup_restores_the_rows_their_owner_and_their_grants() {
    let (source, _source_server) = database_with_rows();
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("anima.backup");

    write(&source.tree, &source.layout, &file).unwrap();

    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::read_dir(folder.path()).unwrap().count(),
        1,
        "no part file is left"
    );
    let target = Database::new();
    let _target_server = target.start();
    roles::prepare(&target.layout).unwrap();

    restore(&target.tree, &target.layout, &file).unwrap();

    assert_eq!(
        target.sql_in("anima", ROWS).unwrap(),
        "500 125250 anima_owner true"
    );
    assert_eq!(target.sql_in("anima", ROWS), source.sql_in("anima", ROWS));
}

#[test]
fn a_restore_into_a_database_that_is_not_empty_changes_nothing() {
    let (database, _server) = database_with_rows();
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("anima.backup");
    write(&database.tree, &database.layout, &file).unwrap();
    database
        .sql_in("anima", "delete from devices.devices where id > 10")
        .unwrap();

    let outcome = restore(&database.tree, &database.layout, &file);

    assert!(
        matches!(outcome, Err(BackupError::Restore { .. })),
        "{outcome:?}"
    );
    assert_eq!(
        database
            .sql_in("anima", "select count(*) from devices.devices")
            .unwrap(),
        "10"
    );
}

#[test]
fn a_backup_with_no_server_fails_and_leaves_no_file() {
    let database = Database::new();
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("anima.backup");

    let outcome = write(&database.tree, &database.layout, &file);

    assert!(
        matches!(outcome, Err(BackupError::Dump { .. })),
        "{outcome:?}"
    );
    assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 0);
}

#[test]
fn a_sound_cluster_passes_the_check() {
    let (database, _server) = database_with_rows();

    assert_eq!(
        check(&database.tree, &database.layout).unwrap(),
        Checked::Sound
    );
}

#[test]
fn a_damaged_index_is_named_by_the_check() {
    let (database, server) = database_with_rows();
    let index_file = database
        .sql_in(
            "anima",
            "select pg_relation_filepath('devices.devices_pkey')",
        )
        .unwrap();
    server.stop(crate::scratch::LIMITS).unwrap();
    // Zeroes over the second page of the index: its items are gone, the page still reads.
    let mut index = fs::OpenOptions::new()
        .write(true)
        .open(database.layout.cluster().join(index_file))
        .unwrap();
    index.seek(SeekFrom::Start(8192)).unwrap();
    index.write_all(&[0_u8; 8192]).unwrap();
    drop(index);
    let _server = database.start();

    let found = check(&database.tree, &database.layout).unwrap();

    let Checked::Damaged(names) = found else {
        panic!("the damage was not found");
    };
    assert!(names.contains("anima.devices.devices_pkey"), "{names:?}");
}

#[test]
fn a_check_with_no_server_is_an_error_not_a_sound_cluster() {
    let database = Database::new();

    let outcome = check(&database.tree, &database.layout);

    assert!(
        matches!(outcome, Err(BackupError::Check { .. })),
        "{outcome:?}"
    );
}

#[test]
fn log_007_only_the_names_of_damaged_relations_are_kept() {
    let report = "btree index \"anima.checks.checks_pkey\":\n    \
        ERROR:  item order invariant violated for index \"checks_pkey\"\n    \
        DETAIL:  Lower index tid=(1,2) (points to heap tid=(0,1)) higher index tid=(1,3)\n\
        heap table \"anima.devices.devices\", block 3, offset 7:\n    xmin 12 precedes\n";

    let names: Vec<String> = damaged_in(report).into_iter().collect();

    assert_eq!(names, ["anima.checks.checks_pkey", "anima.devices.devices"]);
}
