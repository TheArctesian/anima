//! Signing in with TOTP through `/graphql`, and `operator totp` (SEC-002, OD-2, D-8, D-9,
//! LOG-007). Codes come from an RFC 6238 oracle written here, apart from the hub's own.

use std::{
    fmt::Write as _,
    io::{self, Write as _},
    process::Command,
    sync::{Arc, Mutex},
};

use anima_hub::{access, events::process_log};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use hmac::{Hmac, KeyInit as _, Mac as _};
use serde_json::{Value, json};
use sha1::Sha1;
use sqlx::PgPool;

use crate::{
    hub::{
        Hub, PASSWORD, USERNAME, error, set_operator, start, test_vault, unauthenticated,
        wrong_credentials,
    },
    start::url_for,
};

/// Enrols TOTP for the fixture operator; returns the secret read back from the URI.
async fn enrol(pool: &PgPool) -> Vec<u8> {
    set_operator(pool).await;
    let enrolment = access::enrol_totp(pool, &test_vault(), USERNAME)
        .await
        .unwrap();
    secret_of(enrolment.uri())
}

/// The `secret` parameter of an `otpauth://` URI, base32-decoded (RFC 4648).
fn secret_of(uri: &str) -> Vec<u8> {
    let encoded = uri
        .split(['?', '&'])
        .find_map(|part| part.strip_prefix("secret="))
        .expect("a secret parameter");
    let (mut buffer, mut bits, mut out) = (0_u32, 0, Vec::new());
    for c in encoded.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'2'..=b'7' => c - b'2' + 26,
            _ => panic!("not base32: {c}"),
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).unwrap());
        }
    }
    out
}

/// The RFC 6238 code (HMAC-SHA-1, 6 digits, 30 s) for `secret` at `at`.
fn code(secret: &[u8], at: DateTime<Utc>) -> String {
    let counter = u64::try_from(at.timestamp() / 30).unwrap();
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).unwrap();
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[19] & 0x0f);
    let word = u32::from_be_bytes(digest[offset..offset + 4].try_into().unwrap());
    format!("{:06}", (word & 0x7fff_ffff) % 1_000_000)
}

async fn login(hub: &Hub, password: &str, totp_code: Option<&str>) -> Value {
    let query = "mutation($input: LoginInput!) { \
        login(input: $input) { token operator { totpEnabled } } }";
    let input = json!({ "username": USERNAME, "password": password, "totpCode": totp_code });
    hub.post(None, query, json!({ "input": input })).await
}

fn signed_in(response: &Value) -> bool {
    response["data"]["login"]["token"].is_string()
}

fn assert_refused_like_a_wrong_password(response: &Value) {
    assert_eq!(response["data"], json!(null), "{response}");
    assert_eq!(error(response), wrong_credentials());
    assert_eq!(
        response["errors"][0]["extensions"],
        json!({ "code": "UNAUTHENTICATED" })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_an_enrolled_operator_signs_in_with_password_and_current_code(pool: PgPool) {
    let secret = enrol(&pool).await;
    let hub = Hub::new(&pool, start());
    let response = login(&hub, PASSWORD, Some(&code(&secret, start()))).await;
    assert!(signed_in(&response), "{response}");
    assert_eq!(response["data"]["login"]["operator"]["totpEnabled"], true);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_without_totp_the_password_alone_signs_in_and_a_code_is_ignored(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let response = login(&hub, PASSWORD, None).await;
    assert_eq!(response["data"]["login"]["operator"]["totpEnabled"], false);
    assert!(signed_in(&login(&hub, PASSWORD, Some("000000")).await));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_missing_wrong_or_malformed_code_is_refused_like_a_wrong_password(pool: PgPool) {
    let secret = enrol(&pool).await;
    let hub = Hub::new(&pool, start());
    let current = code(&secret, start());
    let wrong = format!("{:06}", (current.parse::<u32>().unwrap() + 1) % 1_000_000);
    let two_steps_late = code(&secret, start() - TimeDelta::seconds(60));
    for typed in [
        None,
        Some(wrong.as_str()),
        Some(&two_steps_late),
        Some(""),
        Some("abcdef"),
    ] {
        assert_refused_like_a_wrong_password(&login(&hub, PASSWORD, typed).await);
    }
    // Five failures lock this address (D-8); the rest come from another one. A right code
    // with a wrong password is refused the same way, and leaves the code unused.
    let elsewhere = Hub::from_client(&pool, start(), "100.64.0.99");
    let wrong_password = login(&elsewhere, "wrong password", Some(&current)).await;
    assert_refused_like_a_wrong_password(&wrong_password);
    assert!(signed_in(
        &login(&elsewhere, PASSWORD, Some(&current)).await
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_code_works_once_and_never_after_a_later_step(pool: PgPool) {
    let secret = enrol(&pool).await;
    let first = code(&secret, start());
    assert!(signed_in(
        &login(&Hub::new(&pool, start()), PASSWORD, Some(&first)).await
    ));

    let same_step = Hub::new(&pool, start() + TimeDelta::seconds(10));
    assert_refused_like_a_wrong_password(&login(&same_step, PASSWORD, Some(&first)).await);

    let next = start() + TimeDelta::seconds(30);
    let later = Hub::new(&pool, next);
    assert_refused_like_a_wrong_password(&login(&later, PASSWORD, Some(&first)).await);
    assert!(signed_in(
        &login(&later, PASSWORD, Some(&code(&secret, next))).await
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_new_secret_starts_with_no_used_step(pool: PgPool) {
    let secret = enrol(&pool).await;
    let hub = Hub::new(&pool, start());
    assert!(signed_in(
        &login(&hub, PASSWORD, Some(&code(&secret, start()))).await
    ));

    let renewed = access::enrol_totp(&pool, &test_vault(), USERNAME)
        .await
        .unwrap();
    let renewed = secret_of(renewed.uri());
    assert_ne!(renewed, secret);
    assert_refused_like_a_wrong_password(
        &login(&hub, PASSWORD, Some(&code(&secret, start()))).await,
    );
    assert!(signed_in(
        &login(&hub, PASSWORD, Some(&code(&renewed, start()))).await
    ));
    access::disable_totp(&pool, USERNAME).await.unwrap();
    assert!(signed_in(&login(&hub, PASSWORD, None).await));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_two_sign_ins_racing_with_one_code_let_only_one_in(pool: PgPool) {
    let secret = enrol(&pool).await;
    let typed = code(&secret, start());
    let (a, b) = (Hub::new(&pool, start()), Hub::new(&pool, start()));
    let (first, second) = tokio::join!(
        login(&a, PASSWORD, Some(&typed)),
        login(&b, PASSWORD, Some(&typed))
    );
    let wins = [&first, &second]
        .into_iter()
        .filter(|r| signed_in(r))
        .count();
    assert_eq!(wins, 1, "{first} {second}");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_wrong_codes_count_toward_the_throttle(pool: PgPool) {
    let secret = enrol(&pool).await;
    let hub = Hub::new(&pool, start());
    for _ in 0..5 {
        assert_refused_like_a_wrong_password(&login(&hub, PASSWORD, Some("000000")).await);
    }
    let right = code(&secret, start());
    assert_refused_like_a_wrong_password(&login(&hub, PASSWORD, Some(&right)).await);

    let rows: Vec<(bool, bool)> =
        sqlx::query_as("select ok, refused from access.login_attempts order by id")
            .fetch_all(&pool)
            .await
            .unwrap();
    let failures = rows.iter().filter(|row| **row == (false, false)).count();
    assert_eq!((rows.len(), failures), (6, 5), "the sixth was refused");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_enrolment_signs_every_client_out(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    access::enrol_totp(&pool, &test_vault(), USERNAME)
        .await
        .unwrap();
    let me = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(error(&me), unauthenticated());
}

/// Collects everything the process log writes.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_007_the_secret_and_codes_reach_no_log_event_or_audit_entry(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    set_operator(&pool).await;
    let enrolment = access::enrol_totp(&pool, &test_vault(), USERNAME)
        .await
        .unwrap();
    let secret = secret_of(enrolment.uri());
    let encoded = enrolment.uri().split("secret=").nth(1).unwrap()[..32].to_owned();
    let hub = Hub::new(&pool, start());
    let mut signals = hub.bus.subscribe();
    let typed = code(&secret, start());
    for attempt in [
        Some("999999"),
        None,
        Some(typed.as_str()),
        Some(typed.as_str()),
    ] {
        login(&hub, PASSWORD, attempt).await;
    }

    let mut written = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    while let Ok(signal) = signals.try_recv() {
        writeln!(written, "{signal:?}").unwrap();
    }
    let stored: Vec<String> = sqlx::query_scalar(
        "select message from events.event_log union all \
         select change::text from events.audit_log",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored.len(),
        5,
        "four events and one audit entry: {stored:?}"
    );
    written.push_str(&stored.join("\n"));
    let hex = secret.iter().fold(String::new(), |mut hex, byte| {
        write!(hex, "{byte:02x}").unwrap();
        hex
    });
    for needle in [
        encoded.as_str(),
        &hex,
        "otpauth",
        "999999",
        &format!("\"{typed}\""),
    ] {
        assert!(!written.contains(needle), "{needle} in {written}");
    }
}

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
