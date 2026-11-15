//! What reaches the server log (LOG-007).

use std::fs;

use super::mode_of;
use crate::Password;
use crate::scratch::Database;

#[test]
fn log_007_no_statement_and_no_error_of_a_session_reaches_the_server_log() {
    let database = Database::new();
    let _server = database.start();
    database
        .sql("create table keyed (k text primary key)")
        .unwrap();
    database
        .sql("insert into keyed values ('a-secret-key')")
        .unwrap();

    database
        .sql("create role probe password 'a-secret-in-a-statement'")
        .unwrap();
    for failing in [
        "alter role nobody_here password 'a-secret-in-a-failed-statement'",
        "create role other password a_secret_unquoted",
        "select 'a-secret-cast'::uuid",
        "insert into keyed values ('a-secret-key')",
    ] {
        database.sql(failing).unwrap_err();
    }

    let log = fs::read_to_string(database.layout.server_log()).unwrap();
    assert!(
        log.contains("ready to accept connections"),
        "starts are logged"
    );
    assert!(!log.contains("secret"), "{log}");
    assert!(!log.contains("nobody_here"), "{log}");
}

#[test]
fn a_refused_login_is_in_the_server_log() {
    let mut database = Database::new();
    let _server = database.start();
    database.password = Password::from_bytes(&[8; 24]);

    database.sql("select 1").unwrap_err();

    let log = fs::read_to_string(database.layout.server_log()).unwrap();
    assert!(log.contains("password authentication failed"), "{log}");
    assert!(!log.contains(database.password.expose()));
}

#[test]
fn log_007_the_hub_login_cannot_turn_the_logging_on() {
    let database = Database::new();
    let _server = database.start();
    database.sql("create role plain login").unwrap();

    for setting in [
        "log_statement = 'all'",
        "log_min_messages = 'debug1'",
        "log_min_error_statement = 'error'",
    ] {
        let refusal = database
            .sql(&format!("set role plain; set {setting}"))
            .unwrap_err();
        assert!(
            refusal.contains("permission denied"),
            "{setting}: {refusal}"
        );
    }
}

#[test]
fn a_large_server_log_moves_aside_at_the_next_start() {
    let database = Database::new();
    let log = database.layout.server_log();
    fs::write(&log, vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();

    let _server = database.start();

    let older = log.with_file_name("server.log.1");
    assert_eq!(fs::metadata(&older).unwrap().len(), 8 * 1024 * 1024 + 1);
    assert!(fs::metadata(&log).unwrap().len() < 1024 * 1024);
    assert_eq!(mode_of(&log), 0o600);
}
