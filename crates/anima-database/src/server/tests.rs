//! Each test makes its own cluster and runs a real server from this computer's PostgreSQL 17.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, SystemTime};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

mod left_over;
mod logging;

use super::pid_file::PidFile;
use super::{Limits, Server, ServerError, Settings, Stopped, lock, stop_left_over};
use crate::scratch::{Database, LIMITS, SETTINGS};
use crate::{Layout, Password, Role};

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn pid_file_text(process_id: u32, started_at: u64, status: &str) -> String {
    format!("{process_id}\n/cluster\n{started_at}\n5432\n/s\n\n  5432001    65536\n{status}\n")
}

#[test]
fn db_001_a_ready_server_listens_on_the_private_socket_only() {
    let database = Database::new();

    let server = database.start();

    assert_eq!(database.sql("show listen_addresses").unwrap(), "");
    assert!(database.layout.socket_file().exists());
    assert_eq!(mode_of(database.layout.socket_folder()), 0o700);
    assert_eq!(mode_of(&database.layout.socket_file()), 0o700);
    assert_eq!(mode_of(&database.layout.server_log()), 0o600);
    assert_eq!(server.stop(LIMITS).unwrap(), Stopped::Clean);
    assert!(!database.pid_file().exists());
    assert!(database.sql("select 1").is_err());
}

#[test]
fn db_003_the_superuser_signs_in_with_the_password_it_was_made_with_and_no_other() {
    let mut database = Database::new();
    let _server = database.start();
    assert_eq!(
        database.sql("select current_user").unwrap(),
        Role::Superuser.login()
    );

    database.password = Password::from_bytes(&[8; 24]);

    let refusal = database.sql("select 1").unwrap_err();
    assert!(
        refusal.contains("password authentication failed"),
        "{refusal}"
    );
}

#[test]
fn the_cluster_has_the_choices_that_cannot_change_later() {
    let database = Database::new();
    let _server = database.start();

    let choices = database.sql(
        "select pg_encoding_to_char(encoding) || ' ' || datlocprovider::text || ' ' || datlocale \
         from pg_database where datname = 'postgres'",
    );

    assert_eq!(choices.unwrap(), "UTF8 b C.UTF-8");
    assert_eq!(database.sql("show data_checksums").unwrap(), "on");
}

#[test]
fn the_drive_cache_setting_reaches_the_server() {
    let database = Database::new();
    let settings = Settings {
        flush_through_drive_cache: true,
        ..SETTINGS
    };
    // Linux has no fsync_writethrough; there the server refuses the setting and does not start.
    if cfg!(target_os = "macos") {
        let mut server =
            Server::start(&database.lock, &database.tree, &database.layout, settings).unwrap();
        server.wait_until_ready(&database.layout, LIMITS).unwrap();

        assert_eq!(
            database.sql("show wal_sync_method").unwrap(),
            "fsync_writethrough"
        );
    }
    let _server = database.start();
    assert_ne!(
        database.sql("show wal_sync_method").unwrap(),
        "fsync_writethrough"
    );
}

#[test]
fn a_second_copy_is_refused_while_the_first_holds_the_lock() {
    let folder = tempfile::tempdir().unwrap();
    let layout = Layout::new(folder.path().join("db"), folder.path().join("s")).unwrap();

    let first = lock(&layout).unwrap();

    assert!(matches!(lock(&layout), Err(ServerError::AnotherCopy)));
    drop(first);
    assert!(lock(&layout).is_ok());
}

#[test]
fn every_acknowledged_row_survives_a_killed_server() {
    let database = Database::new();
    let mut server = database.start();
    database
        .sql("create table marks (n int primary key)")
        .unwrap();

    let killer = server.process_id();
    let acknowledged = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut count = 0;
            while database
                .sql(&format!("insert into marks values ({count})"))
                .is_ok()
            {
                count += 1;
            }
            count
        });
        std::thread::sleep(Duration::from_millis(700));
        kill(
            Pid::from_raw(i32::try_from(killer).unwrap()),
            Signal::SIGKILL,
        )
        .unwrap();
        writer.join().unwrap()
    });
    while server.ended().unwrap().is_none() {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        acknowledged > 0,
        "the writer committed nothing before the kill"
    );
    assert!(
        database.pid_file().exists(),
        "a killed server leaves its file"
    );

    assert!(!stop_left_over(&database.lock, &database.tree, &database.layout, LIMITS).unwrap());
    let _server = database.start();

    // The last insert can be committed and the kill can take its answer.
    let rows: i64 = database
        .sql("select count(*) from marks")
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        rows == acknowledged || rows == acknowledged + 1,
        "{rows} rows for {acknowledged} acknowledged"
    );
}

#[test]
fn a_child_that_fails_is_not_ready_because_another_server_answers() {
    let database = Database::new();
    let _running = database.start();

    let mut second =
        Server::start(&database.lock, &database.tree, &database.layout, SETTINGS).unwrap();
    let outcome = second.wait_until_ready(&database.layout, LIMITS);

    assert!(
        matches!(outcome, Err(ServerError::EndedDuringStart { .. })),
        "{outcome:?}"
    );
    assert_eq!(database.sql("select 1").unwrap(), "1");
}

#[test]
fn a_dropped_server_does_not_keep_running() {
    let database = Database::new();

    drop(database.start());

    assert!(!database.pid_file().exists());
    assert!(database.sql("select 1").is_err());
}

#[test]
fn a_pid_file_is_ready_only_for_the_child_that_wrote_it() {
    let now = SystemTime::now();
    let at = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let ready = |text: &str, process_id| {
        PidFile::parse(text).is_some_and(|file| file.is_ready_for(process_id, now))
    };

    assert!(ready(&pid_file_text(41, at, "ready"), 41));
    assert!(ready(&pid_file_text(41, at + 2, "ready   "), 41));
    assert!(!ready(&pid_file_text(41, at, "starting"), 41));
    assert!(!ready(&pid_file_text(41, at, "stopping"), 41));
    assert!(
        !ready(&pid_file_text(40, at, "ready"), 41),
        "another server"
    );
    assert!(
        !ready(&pid_file_text(41, at - 1, "ready"), 41),
        "a file from before the start, with a number that came round again"
    );
}

#[test]
fn a_pid_file_that_is_not_whole_yet_says_nothing() {
    assert_eq!(PidFile::parse(""), None);
    assert_eq!(PidFile::parse("41\n/cluster\n"), None);
    assert_eq!(
        PidFile::parse("41\n/cluster\n1794000000\n5432\n/s\n\n  1 2\n"),
        None
    );
    assert_eq!(
        PidFile::parse(&pid_file_text(41, 5, "ready").replace("41", "x")),
        None
    );
}

#[test]
fn a_server_that_does_not_stop_in_time_is_told_to_quit_and_recovers_at_the_next_start() {
    let database = Database::new();
    let server = database.start();
    database.sql("create table kept as select 1 as n").unwrap();
    let no_time = Limits {
        stop: Duration::ZERO,
        ..LIMITS
    };

    assert_eq!(server.stop(no_time).unwrap(), Stopped::Immediate);

    let _server = database.start();
    assert_eq!(database.sql("select n from kept").unwrap(), "1");
    let log = fs::read_to_string(database.layout.server_log()).unwrap();
    assert!(log.contains("automatic recovery in progress"), "{log}");
}
