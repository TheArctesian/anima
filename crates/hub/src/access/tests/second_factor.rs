//! The TOTP secret at rest: sealed with the vault for its operator, and a value that does not
//! open is a hub fault, never a sign-in (SEC-002, R-010, LOG-007).

use std::net::{IpAddr, Ipv4Addr};

use sqlx::PgPool;

use super::{now, test_vault};
use crate::{
    access::{
        ClientAddr, Error, Password, disable_totp, enrol_totp,
        login::{self, Attempt, Issue},
        set_operator, store,
        totp::{context, otpauth_uri},
    },
    credentials::Sealed,
    events::Bus,
};

const USERNAME: &str = "operator";
const PASSWORD: &str = "correct horse battery staple";

async fn operator(pool: &PgPool) {
    set_operator(pool, USERNAME, Password::new(PASSWORD.to_owned()), now())
        .await
        .unwrap();
}

fn attempt(code: Option<&str>) -> Attempt {
    Attempt {
        username: USERNAME.to_owned(),
        password: Password::new(PASSWORD.to_owned()),
        totp_code: code.map(str::to_owned),
        client: ClientAddr(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 9))),
        at: now(),
        issue: Issue::LOGIN,
    }
}

async fn sealed(pool: &PgPool) -> (uuid::Uuid, Vec<u8>) {
    sqlx::query_as("select id, totp_secret_enc from access.operators")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_r_010_the_secret_is_stored_sealed_for_its_operator(pool: PgPool) {
    operator(&pool).await;
    let vault = test_vault(7);
    let enrolment = enrol_totp(&pool, &vault, USERNAME).await.unwrap();

    let (id, stored) = sealed(&pool).await;
    let sealed = Sealed::from_bytes(&stored).unwrap();
    let secret = vault.open(&sealed, context(id.into()).as_bytes()).unwrap();
    assert_eq!(secret.expose().len(), 20);
    assert_eq!(
        otpauth_uri(secret.expose(), USERNAME).as_str(),
        enrolment.uri()
    );
    assert!(
        !stored
            .windows(secret.expose().len())
            .any(|w| w == secret.expose())
    );
    assert!(vault.open(&sealed, b"access.operators.totp:").is_err());
    assert!(vault.open(&sealed, b"").is_err());
}

/// The client's view of a refusal: message and extensions, nothing else.
fn shown(error: Error) -> (String, Option<async_graphql::ErrorExtensionValues>) {
    let shown = error.into_response_error();
    (shown.message, shown.extensions)
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_r_010_a_secret_from_another_key_is_answered_like_a_wrong_password(pool: PgPool) {
    operator(&pool).await;
    enrol_totp(&pool, &test_vault(7), USERNAME).await.unwrap();
    let (other, bus) = (test_vault(8), Bus::new(4));

    let mut wrong_password = attempt(Some("123456"));
    wrong_password.password = Password::new("wrong password guess".to_owned());
    let wrong = login::login(&pool, &bus, &other, wrong_password)
        .await
        .err()
        .unwrap();
    let right = login::login(&pool, &bus, &other, attempt(Some("123456"))).await;
    let right = right.err().expect("no sign-in");
    assert!(matches!(wrong, Error::WrongCredentials));
    assert!(matches!(right, Error::TotpSecret(_)));
    assert_eq!(
        right.fault(),
        Some("totp_secret"),
        "the edge logs the fault"
    );
    assert_eq!(
        shown(right),
        shown(wrong),
        "the answer hides the right password"
    );

    assert_eq!(attempts(&pool).await, [(false, false); 2], "both counted");
    let events: Vec<(String, String)> =
        sqlx::query_as("select severity, message from events.event_log order by at, id")
            .fetch_all(&pool)
            .await
            .unwrap();
    let failed = "Sign-in failed for operator operator from 100.64.0.9.";
    let unchecked = "The second factor of operator operator cannot be checked: its TOTP secret \
                     does not open with the hub key. Enrol it again with anima-hub operator totp.";
    let expected = [
        ("warning", failed),
        ("warning", failed),
        ("error", unchecked),
    ];
    let expected = expected.map(|(severity, message)| (severity.to_owned(), message.to_owned()));
    assert_eq!(events, expected);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_malformed_code_is_refused_before_the_secret_is_opened(pool: PgPool) {
    operator(&pool).await;
    enrol_totp(&pool, &test_vault(7), USERNAME).await.unwrap();
    let other = test_vault(8);
    for code in ["x", "12345", "1234567", "12345a", ""] {
        let result = login::login(&pool, &Bus::new(4), &other, attempt(Some(code))).await;
        assert!(matches!(result, Err(Error::WrongCredentials)), "{code:?}");
    }
    let unchecked: i64 =
        sqlx::query_scalar("select count(*) from events.event_log where severity = 'error'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(unchecked, 0, "the vault was never asked");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_used_step_needs_a_secret(pool: PgPool) {
    operator(&pool).await;
    let error = sqlx::query("update access.operators set totp_last_step = 5")
        .execute(&pool)
        .await
        .unwrap_err();
    let constraint = error.as_database_error().and_then(|e| e.constraint());
    assert_eq!(constraint, Some("operators_totp_step_needs_secret"));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_r_010_a_secret_moved_to_another_operator_does_not_open(pool: PgPool) {
    operator(&pool).await;
    let vault = test_vault(7);
    enrol_totp(&pool, &vault, USERNAME).await.unwrap();
    sqlx::query("update access.operators set id = '0190a000-0000-7000-8000-0000000000ff'")
        .execute(&pool)
        .await
        .unwrap();

    let result = login::login(&pool, &Bus::new(4), &vault, attempt(Some("123456"))).await;
    assert!(matches!(result, Err(Error::TotpSecret(_))));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_missing_code_is_refused_before_the_secret_is_opened(pool: PgPool) {
    operator(&pool).await;
    enrol_totp(&pool, &test_vault(7), USERNAME).await.unwrap();
    let result = login::login(&pool, &Bus::new(4), &test_vault(8), attempt(None)).await;
    assert!(matches!(result, Err(Error::WrongCredentials)));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_disabling_removes_the_secret_and_the_password_signs_in(pool: PgPool) {
    operator(&pool).await;
    let vault = test_vault(7);
    enrol_totp(&pool, &vault, USERNAME).await.unwrap();
    disable_totp(&pool, USERNAME).await.unwrap();

    let stored: (Option<Vec<u8>>, Option<i64>) =
        sqlx::query_as("select totp_secret_enc, totp_last_step from access.operators")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, (None, None));
    let result = login::login(&pool, &Bus::new(4), &vault, attempt(None)).await;
    assert!(result.is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_enrolment_needs_an_existing_operator(pool: PgPool) {
    operator(&pool).await;
    let vault = test_vault(7);
    for result in [
        enrol_totp(&pool, &vault, "nobody").await.err(),
        disable_totp(&pool, "nobody").await.err(),
    ] {
        assert!(
            matches!(
                result,
                Some(Error::Validation {
                    field: "username",
                    ..
                })
            ),
            "{result:?}"
        );
    }
    let (_, stored): (uuid::Uuid, Option<Vec<u8>>) =
        sqlx::query_as("select id, totp_secret_enc from access.operators")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_step_is_claimed_only_for_the_secret_that_was_checked(pool: PgPool) {
    operator(&pool).await;
    enrol_totp(&pool, &test_vault(7), USERNAME).await.unwrap();
    let (id, stored) = sealed(&pool).await;
    let id = id.into();
    let replaced = [stored.as_slice(), b"x"].concat();
    assert!(
        !store::claim_totp_step(&pool, id, &replaced, 9)
            .await
            .unwrap()
    );
    assert!(store::claim_totp_step(&pool, id, &stored, 9).await.unwrap());
    assert!(!store::claim_totp_step(&pool, id, &stored, 9).await.unwrap());
    assert!(!store::claim_totp_step(&pool, id, &stored, 8).await.unwrap());
    assert!(
        store::claim_totp_step(&pool, id, &stored, 10)
            .await
            .unwrap()
    );
}

/// `(ok, refused)` of every login attempt, oldest first.
async fn attempts(pool: &PgPool) -> Vec<(bool, bool)> {
    sqlx::query_as("select ok, refused from access.login_attempts order by id")
        .fetch_all(pool)
        .await
        .unwrap()
}
