//! An account change that commits during a sign-in leaves no token it should have deleted,
//! and one TOTP step goes to one sign-in only (SEC-002, D-9). From the T6 security review.

use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use sqlx::PgPool;

use super::{now, test_vault};
use crate::{
    access::{
        ClientAddr, Password, disable_totp, enrol_totp,
        login::{self, Attempt},
        set_operator,
        store::{self, Account},
    },
    events::Bus,
};

const USERNAME: &str = "operator";
const PASSWORD: &str = "correct horse battery staple";

/// Delays before the account change, so it lands before, during and after the password check.
const DELAYS_MS: [u64; 6] = [0, 1, 2, 5, 10, 40];

fn attempt(password: &str) -> Attempt {
    Attempt {
        username: USERNAME.to_owned(),
        password: Password::new(password.to_owned()),
        totp_code: None,
        client: ClientAddr(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 3))),
        at: now(),
    }
}

async fn operator(pool: &PgPool, password: &str) {
    set_operator(pool, USERNAME, Password::new(password.to_owned()), now())
        .await
        .unwrap();
}

async fn tokens(pool: &PgPool) -> i64 {
    sqlx::query_scalar("select count(*) from access.api_tokens")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn account(pool: &PgPool) -> Account {
    store::account_by_username(pool, USERNAME)
        .await
        .unwrap()
        .unwrap()
}

/// Starts a password-only sign-in, waits `delay_ms`, then runs `change`; returns the number
/// of tokens left once both are done.
async fn race<F: Future<Output = ()>>(pool: &PgPool, delay_ms: u64, change: F) -> i64 {
    let signing_in = tokio::spawn({
        let pool = pool.clone();
        async move {
            let vault = test_vault(7);
            let _ = login::login(&pool, &Bus::new(4), &vault, attempt(PASSWORD)).await;
        }
    });
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    change.await;
    signing_in.await.unwrap();
    tokens(pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_an_enrolment_during_a_sign_in_leaves_no_password_only_token(pool: PgPool) {
    operator(&pool, PASSWORD).await;
    let vault = test_vault(7);
    for delay_ms in DELAYS_MS {
        disable_totp(&pool, USERNAME).await.unwrap();
        let left = race(&pool, delay_ms, async {
            enrol_totp(&pool, &vault, USERNAME).await.unwrap();
        })
        .await;
        assert_eq!(
            left, 0,
            "a password-only token survived enrolment ({delay_ms} ms)"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_9_operator_set_during_a_sign_in_leaves_no_token_for_the_old_password(
    pool: PgPool,
) {
    for delay_ms in DELAYS_MS {
        operator(&pool, PASSWORD).await;
        let left = race(&pool, delay_ms, operator(&pool, "a new long passphrase")).await;
        assert_eq!(
            left, 0,
            "a token for the old password survived ({delay_ms} ms)"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_an_account_changed_since_its_check_is_not_locked_as_checked(pool: PgPool) {
    operator(&pool, PASSWORD).await;
    let checked = account(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    assert!(store::lock_unchanged(&mut conn, &checked).await.unwrap());
    drop(conn);

    enrol_totp(&pool, &test_vault(7), USERNAME).await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    assert!(
        !store::lock_unchanged(&mut conn, &checked).await.unwrap(),
        "secret changed"
    );
    let enrolled = account(&pool).await;
    assert!(store::lock_unchanged(&mut conn, &enrolled).await.unwrap());

    operator(&pool, PASSWORD).await;
    assert!(
        !store::lock_unchanged(&mut conn, &enrolled).await.unwrap(),
        "hash changed"
    );
    let renamed = Account {
        username: "someone".to_owned(),
        ..account(&pool).await
    };
    assert!(
        !store::lock_unchanged(&mut conn, &renamed).await.unwrap(),
        "name changed"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_of_many_concurrent_claims_of_one_step_one_wins(pool: PgPool) {
    operator(&pool, PASSWORD).await;
    enrol_totp(&pool, &test_vault(7), USERNAME).await.unwrap();
    let enrolled = account(&pool).await;
    let sealed = enrolled.totp_secret_enc.unwrap();
    let mut claims = tokio::task::JoinSet::new();
    for _ in 0..32 {
        let (pool, sealed) = (pool.clone(), sealed.clone());
        claims.spawn(async move {
            store::claim_totp_step(&pool, enrolled.id, &sealed, 42)
                .await
                .unwrap()
        });
    }
    let wins = claims
        .join_all()
        .await
        .into_iter()
        .filter(|won| *won)
        .count();
    assert_eq!(wins, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_the_checked_account_stays_locked_until_the_token_is_issued(pool: PgPool) {
    operator(&pool, PASSWORD).await;
    let checked = account(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    assert!(store::lock_unchanged(&mut tx, &checked).await.unwrap());

    let error = sqlx::query("select id from access.operators for update nowait")
        .execute(&pool)
        .await
        .unwrap_err();
    let code = error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code);
    assert_eq!(code.as_deref(), Some("55P03"), "lock_not_available");
    tx.rollback().await.unwrap();
}
