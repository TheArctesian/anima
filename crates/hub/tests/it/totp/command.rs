//! `anima-hub operator totp`: the URI goes only to a terminal, and `--disable` turns the
//! second factor off (SEC-002, D-9, LOG-007).

use std::{io::Write as _, process::Command};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::PgPool;

use super::enrol;
use crate::{
    hub::{USERNAME, set_operator},
    start::url_for,
};

/// Runs `anima-hub` with the test database and a throwaway key; returns (success, stdout,
/// stderr). Standard output is a pipe, never a terminal.
fn hub_command(pool: &PgPool, args: &[&str]) -> (bool, String, String) {
    let database = pool.connect_options().get_database().unwrap().to_owned();
    let mut url_file = tempfile::NamedTempFile::new().unwrap();
    write!(url_file, "{}", url_for(&database)).unwrap();
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    write!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_anima-hub"))
        .env_clear()
        .env("ANIMA_KEY_FILE", key_file.path())
        .env("ANIMA_DATABASE_URL_FILE", url_file.path())
        .env("ANIMA_LOG", "error")
        .env("ANIMA_LISTEN", "127.0.0.1:0")
        .args(args)
        .output()
        .unwrap();
    let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
    (
        output.status.success(),
        text(output.stdout),
        text(output.stderr),
    )
}

async fn enrolled(pool: &PgPool) -> bool {
    sqlx::query_scalar("select totp_secret_enc is not null from access.operators")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_operator_totp_refuses_to_print_the_uri_into_a_pipe(pool: PgPool) {
    set_operator(&pool).await;
    let (ok, stdout, stderr) = hub_command(&pool, &["operator", "totp", USERNAME]);
    assert!(!ok);
    assert_eq!(stdout, "");
    assert_eq!(
        stderr.trim(),
        "anima-hub: run operator totp on a terminal: the enrolment URI is shown only there"
    );
    assert!(!enrolled(&pool).await, "nothing was enrolled");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_operator_totp_disable_turns_the_second_factor_off(pool: PgPool) {
    enrol(&pool).await;
    let (ok, stdout, stderr) = hub_command(&pool, &["operator", "totp", USERNAME, "--disable"]);
    assert!(ok, "{stderr}");
    assert_eq!(
        stdout.trim(),
        format!("TOTP is off for operator {USERNAME}.")
    );
    assert!(!enrolled(&pool).await);
    let (ok, _, stderr) = hub_command(&pool, &["operator", "totp", "nobody", "--disable"]);
    assert!(!ok);
    assert_eq!(stderr.trim(), "anima-hub: No operator has this username.");
}
