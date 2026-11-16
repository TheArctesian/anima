//! `operator create`: the account and its second factor are saved together, only with a right
//! code, and that code is then used (FRN-004, SEC-002, OD-2).

use anima_hub::fleet::access::{self, NewOperator, Password};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{assert_refused_like_a_wrong_password, code, login, secret_of, signed_in};
use crate::hub::{
    Hub, PASSWORD, USERNAME, error, set_operator, start, test_vault, unauthenticated,
};

/// A new secret from `operator totp --new`: its base32 text and its bytes.
pub(super) fn new_secret() -> (String, Vec<u8>) {
    let made = access::new_totp(USERNAME).unwrap();
    (made.secret().to_owned(), secret_of(made.uri()))
}

fn new_operator(password: &str, secret: &str, typed: &str) -> NewOperator {
    NewOperator {
        username: USERNAME.to_owned(),
        password: Password::new(password.to_owned()),
        secret: Zeroizing::new(secret.to_owned()),
        code: Zeroizing::new(typed.to_owned()),
    }
}

async fn create(pool: &PgPool, new: NewOperator) -> Result<(), access::Error> {
    let created = access::create_operator(pool, &test_vault(), new, start()).await;
    created.map(|_| ())
}

/// `operator create` at [`start`] with this password, secret text and code.
pub(super) async fn create_with(
    pool: &PgPool,
    password: &str,
    secret: &str,
    typed: &str,
) -> Result<(), access::Error> {
    create(pool, new_operator(password, secret, typed)).await
}

/// Makes every later write of a TOTP secret fail inside its transaction: the step after the
/// account row is written.
async fn refuse_totp_writes(pool: &PgPool) {
    for sql in [
        "create function access.refuse_totp() returns trigger language plpgsql as \
         $$ begin raise exception 'refused by the test'; end $$",
        "create trigger refuse_totp before update of totp_secret_enc on access.operators \
         for each row execute function access.refuse_totp()",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

async fn audit_entries(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from events.audit_log")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_004_a_failure_after_the_account_is_written_leaves_nothing(pool: PgPool) {
    refuse_totp_writes(&pool).await;
    let (text, secret) = new_secret();
    let typed = code(&secret, start());
    let failed = create_with(&pool, PASSWORD, &text, &typed).await;
    assert!(
        matches!(failed, Err(access::Error::Database(_))),
        "{failed:?}"
    );
    assert_eq!(
        operators(&pool).await,
        0,
        "no account without its second factor"
    );
    assert_eq!(audit_entries(&pool).await, 0);

    // An existing account keeps its password, its tokens and its sign-ins.
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let before = audit_entries(&pool).await;
    let failed = create_with(&pool, "a recovered long passphrase", &text, &typed).await;
    assert!(failed.is_err());
    assert_eq!(audit_entries(&pool).await, before);
    let me = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(me["data"]["me"]["username"], USERNAME, "{me}");
    assert!(hub.login(USERNAME, PASSWORD).await["data"]["login"]["token"].is_string());
}

async fn operators(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from access.operators")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_frn_004_the_code_that_created_the_account_is_used_and_the_next_one_works(
    pool: PgPool,
) {
    let (text, secret) = new_secret();
    let typed = code(&secret, start());
    create(&pool, new_operator(PASSWORD, &text, &typed))
        .await
        .unwrap();

    let same_step = Hub::new(&pool, start() + TimeDelta::seconds(5));
    assert_refused_like_a_wrong_password(&login(&same_step, PASSWORD, Some(&typed)).await);
    let next = start() + TimeDelta::seconds(30);
    let later = Hub::new(&pool, next);
    assert!(signed_in(
        &login(&later, PASSWORD, Some(&code(&secret, next))).await
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_004_a_wrong_code_a_bad_secret_or_a_short_password_saves_nothing(pool: PgPool) {
    let (text, secret) = new_secret();
    let right = code(&secret, start());
    let wrong = format!("{:06}", (right.parse::<u32>().unwrap() + 1) % 1_000_000);
    let late = code(&secret, start() - TimeDelta::seconds(60));
    let other = new_secret().0;
    let cases = [
        (PASSWORD, text.as_str(), wrong.as_str(), "code"),
        (PASSWORD, &text, &late, "code"),
        (PASSWORD, &other, &right, "code"),
        (PASSWORD, &text[..31], &right, "secret"),
        (PASSWORD, &format!("{text}A"), &right, "secret"),
        (PASSWORD, &format!("1{}", &text[1..]), &right, "secret"),
        ("too short", &text, &right, "password"),
    ];
    for (password, secret_text, typed, field) in cases {
        let refused = create(&pool, new_operator(password, secret_text, typed)).await;
        match refused {
            Err(access::Error::Validation { field: got, .. }) => assert_eq!(got, field),
            other => panic!("{field}: {:?}", other.err()),
        }
    }
    assert_eq!(operators(&pool).await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_004_a_lowercase_secret_is_the_same_secret(pool: PgPool) {
    let (text, secret) = new_secret();
    let typed = code(&secret, start());
    let lower = text.to_ascii_lowercase();
    create(&pool, new_operator(PASSWORD, &lower, &typed))
        .await
        .unwrap();
    let next = start() + TimeDelta::seconds(30);
    let hub = Hub::new(&pool, next);
    assert!(signed_in(
        &login(&hub, PASSWORD, Some(&code(&secret, next))).await
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_004_d_9_create_replaces_the_operator_and_signs_every_client_out(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let (text, secret) = new_secret();
    let recovered = "a recovered long passphrase";
    let typed = code(&secret, start());
    create(&pool, new_operator(recovered, &text, &typed))
        .await
        .unwrap();

    assert_eq!(operators(&pool).await, 1);
    let me = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(error(&me), unauthenticated());
    let next = start() + TimeDelta::seconds(30);
    let later = Hub::new(&pool, next);
    let typed = code(&secret, next);
    assert_refused_like_a_wrong_password(&login(&later, PASSWORD, Some(&typed)).await);
    let response = later
        .post(
            None,
            "mutation($input: LoginInput!) { login(input: $input) { token } }",
            json!({ "input": { "username": USERNAME, "password": recovered, "totpCode": typed } }),
        )
        .await;
    assert!(signed_in(&response), "{response}");
}

#[test]
fn frn_004_a_new_secret_needs_no_operator_and_a_possible_username() {
    let (text, secret) = new_secret();
    assert_eq!(text.len(), 32);
    assert_eq!(secret.len(), 20);
    assert_ne!(new_secret().1, secret, "each secret is new");
    let made = access::new_totp(USERNAME).unwrap();
    assert!(
        made.uri()
            .starts_with("otpauth://totp/Anima:operator?secret=")
    );
    assert!(made.uri().contains(made.secret()));
    for refused in ["", "an operator", "x/y", &"a".repeat(65)] {
        assert!(access::new_totp(refused).is_err(), "{refused:?}");
    }
}
