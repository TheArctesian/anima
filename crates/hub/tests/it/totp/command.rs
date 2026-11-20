//! `anima-hub operator totp`: the URI goes only to a terminal, `--disable` turns the second
//! factor off and `--new` prints a secret for `operator create`; `operator show` (SEC-002,
//! D-9, LOG-007, FRN-003, FRN-004).

use std::{io::Write as _, process::Stdio};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{assert_refused_like_a_wrong_password, code, enrol, login, secret_of, signed_in};
use crate::{
    hub::{Hub, PASSWORD, USERNAME, error, set_operator, unauthenticated},
    start::url_for,
};

/// Runs `anima-hub` with the test database and a throwaway key and no standard input; returns
/// (success, stdout, stderr). Standard output is a pipe, never a terminal.
fn hub_command(pool: &PgPool, args: &[&str]) -> (bool, String, String) {
    hub_command_with(pool, args, b"")
}

/// [`hub_command`] with `input` on standard input.
fn hub_command_with(pool: &PgPool, args: &[&str], input: &[u8]) -> (bool, String, String) {
    run_hub(pool, args, Stdio::piped(), input)
}

/// Runs `anima-hub` with `stdin` as its standard input, and writes `input` to it when it is a
/// pipe.
fn run_hub(pool: &PgPool, args: &[&str], stdin: Stdio, input: &[u8]) -> (bool, String, String) {
    let database = pool.connect_options().get_database().unwrap().to_owned();
    let mut url_file = tempfile::NamedTempFile::new().unwrap();
    write!(url_file, "{}", url_for(&database)).unwrap();
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    write!(key_file, "{}", STANDARD.encode([7_u8; 32])).unwrap();
    let mut child = crate::transport::process::hub()
        .env("ANIMA_KEY_FILE", key_file.path())
        .env("ANIMA_DATABASE_URL_FILE", url_file.path())
        .env("ANIMA_LOG", "error")
        .env("ANIMA_LISTEN", "127.0.0.1:0")
        .args(args)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A command that reads nothing may end first; the write then fails, which is fine.
    if let Some(mut pipe) = child.stdin.take() {
        let _ = pipe.write_all(input);
    }
    let output = child.wait_with_output().unwrap();
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

/// Standard output here is always a pipe, so the success path that prints the URI on a real
/// terminal is not covered by these tests; `access::enrol_totp` and its URI are tested in
/// `totp.rs` and the access unit tests.
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

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_operator_totp_without_a_username_is_a_usage_error(pool: PgPool) {
    set_operator(&pool).await;
    for args in [
        &["operator", "totp", "--disable"][..],
        &["operator", "totp", "--new"],
        &["operator", "totp"],
        &["operator", "create"],
        &["operator", "show", USERNAME],
    ] {
        let (ok, stdout, stderr) = hub_command(&pool, args);
        assert!(!ok);
        assert_eq!(stdout, "");
        assert!(stderr.starts_with("anima-hub: usage: "), "{stderr}");
    }
    assert!(!enrolled(&pool).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_003_operator_show_prints_the_operator_and_its_second_factor(pool: PgPool) {
    let show = |pool: &PgPool| {
        let (ok, stdout, stderr) = hub_command(pool, &["operator", "show"]);
        assert!(ok, "{stderr}");
        assert_eq!(stdout.lines().count(), 1, "{stdout}");
        serde_json::from_str::<Value>(&stdout).unwrap()
    };
    assert_eq!(show(&pool), Value::Null);
    set_operator(&pool).await;
    assert_eq!(show(&pool), json!({ "name": USERNAME, "totp": false }));
    enrol(&pool).await;
    assert_eq!(show(&pool), json!({ "name": USERNAME, "totp": true }));
}

/// `operator totp --new` once, parsed: the secret text and the URI.
fn new_totp(pool: &PgPool, username: &str) -> (String, String) {
    let (ok, stdout, stderr) = hub_command(pool, &["operator", "totp", username, "--new"]);
    assert!(ok, "{stderr}");
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    let line: Value = serde_json::from_str(&stdout).unwrap();
    let field = |name: &str| line[name].as_str().unwrap().to_owned();
    (field("secret"), field("uri"))
}

async fn operators(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from access.operators")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_004_totp_new_prints_a_secret_and_saves_nothing(pool: PgPool) {
    let (secret, uri) = new_totp(&pool, "ada");
    assert_eq!(secret.len(), 32);
    let expected = format!(
        "otpauth://totp/Anima:ada?secret={secret}&issuer=Anima&algorithm=SHA1&digits=6&period=30"
    );
    assert_eq!(uri, expected);
    assert_eq!(operators(&pool).await, 0, "no operator is needed or made");
    set_operator(&pool).await;
    new_totp(&pool, USERNAME);
    assert!(!enrolled(&pool).await);
}

/// `operator create` with three lines on standard input, the second with a CRLF ending.
fn create(pool: &PgPool, password: &str, secret: &str, typed: &str) -> (bool, String, String) {
    let input = format!("{password}\n{secret}\r\n{typed}\n");
    hub_command_with(pool, &["operator", "create", USERNAME], input.as_bytes())
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_004_log_007_operator_create_with_a_wrong_code_saves_nothing_and_echoes_nothing(
    pool: PgPool,
) {
    let (secret, uri) = new_totp(&pool, USERNAME);
    let right = code(&secret_of(&uri), Utc::now());
    let wrong = format!(
        "{:06}",
        (right.parse::<u32>().unwrap() + 500_000) % 1_000_000
    );
    let (ok, stdout, stderr) = create(&pool, PASSWORD, &secret, &wrong);
    assert!(!ok);
    assert_eq!(stdout, "");
    assert_eq!(
        stderr.trim(),
        "anima-hub: The code does not match the secret now. Nothing was saved."
    );
    let (ok, _, stderr) = create(&pool, PASSWORD, &secret[..20], &right);
    assert!(!ok);
    assert!(stderr.contains("32 base32 characters"), "{stderr}");
    assert!(!stderr.contains(&secret[..20]), "{stderr}");
    let (ok, _, stderr) = create(&pool, PASSWORD, &secret, "");
    assert!(!ok, "{stderr}");
    assert_eq!(operators(&pool).await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_frn_004_operator_create_saves_both_factors_and_uses_the_code(pool: PgPool) {
    let (secret, uri) = new_totp(&pool, USERNAME);
    let secret_bytes = secret_of(&uri);
    let now = Utc::now();
    let typed = code(&secret_bytes, now);
    let (ok, stdout, stderr) = create(&pool, PASSWORD, &secret, &typed);
    assert!(ok, "{stderr}");
    assert_eq!(
        stdout.trim(),
        format!("Operator {USERNAME} is set, with TOTP.")
    );
    assert!(enrolled(&pool).await);

    let same = Hub::new(&pool, now);
    assert_refused_like_a_wrong_password(&login(&same, PASSWORD, Some(&typed)).await);
    let next = now + TimeDelta::seconds(30);
    let later = Hub::new(&pool, next);
    let next_code = code(&secret_bytes, next);
    assert!(signed_in(&login(&later, PASSWORD, Some(&next_code)).await));
}

/// The two ends of a new pseudo-terminal: the controller, and the terminal side as a file.
#[cfg(unix)]
fn pseudo_terminal() -> (std::os::fd::OwnedFd, std::fs::File) {
    use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
    let controller = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
    grantpt(&controller).unwrap();
    unlockpt(&controller).unwrap();
    let name = ptsname(&controller, Vec::new()).unwrap();
    let terminal = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(name.to_str().unwrap())
        .unwrap();
    (controller, terminal)
}

#[cfg(unix)]
#[sqlx::test(migrations = "./migrations")]
async fn log_007_operator_create_refuses_a_terminal_that_would_echo_the_secrets(pool: PgPool) {
    let (_controller, terminal) = pseudo_terminal();
    let (ok, stdout, stderr) = run_hub(
        &pool,
        &["operator", "create", USERNAME],
        Stdio::from(terminal),
        b"",
    );
    assert!(!ok);
    assert_eq!(stdout, "");
    assert!(stderr.contains("pipe the three lines in"), "{stderr}");
    assert_eq!(operators(&pool).await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_9_operator_totp_disable_signs_every_client_out(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, Utc::now());
    let token = hub.token().await;
    let (ok, _, stderr) = hub_command(&pool, &["operator", "totp", USERNAME, "--disable"]);
    assert!(ok, "{stderr}");
    let me = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(error(&me), unauthenticated());
}
