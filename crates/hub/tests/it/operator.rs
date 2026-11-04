//! `operator set` clearing the way back in, and retention at its boundaries (D-8, D-9, K2).

use anima_hub::access::{self, Password};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use crate::hub::{CLIENT, Hub, PASSWORD, USERNAME, error, set_operator, start, wrong_credentials};

async fn count(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_9_a_rename_clears_the_old_name_and_its_locked_address(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    for _ in 0..5 {
        hub.login(USERNAME, "wrong password").await;
    }
    assert_eq!(
        error(&hub.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );

    let renamed = "a renamed long passphrase";
    access::set_operator(&pool, "newname", Password::new(renamed.to_owned()), start())
        .await
        .unwrap();
    let old = count(
        &pool,
        "select count(*) from access.login_attempts where username = 'operator'",
    );
    assert_eq!(old.await, 0);
    let response = hub.login("newname", renamed).await;
    assert!(response["data"]["login"]["token"].is_string(), "{response}");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_9_operator_set_clears_the_addresses_its_name_was_tried_from(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    hub.login(USERNAME, "wrong password").await;
    for _ in 0..5 {
        hub.login("opertor", PASSWORD).await;
    }
    let elsewhere = Hub::from_client(&pool, start(), "100.64.0.99");
    elsewhere.login("someone", "guess").await;
    assert_eq!(
        error(&hub.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );

    set_operator(&pool).await;
    let left: Vec<(String, String)> =
        sqlx::query_as("select username, host(client_addr) from access.login_attempts")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(left, [("someone".to_owned(), "100.64.0.99".to_owned())]);
    assert!(hub.login(USERNAME, PASSWORD).await["data"]["login"]["token"].is_string());
}

async fn insert_attempt(pool: &PgPool, at: DateTime<Utc>) {
    sqlx::query(
        "insert into access.login_attempts (id, at, username, client_addr, ok) \
         values (gen_random_uuid(), $1, 'operator', $2::inet, false)",
    )
    .bind(at)
    .bind(CLIENT)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_token(pool: &PgPool, fill: u8, expires_at: DateTime<Utc>) {
    sqlx::query(
        "insert into access.api_tokens (id, operator_id, token_hash, scope, expires_at, created_at) \
         select gen_random_uuid(), id, $1, 'write', $2, $3 from access.operators",
    )
    .bind(vec![fill; 32])
    .bind(expires_at)
    .bind(start())
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_prune_keeps_89_days_of_attempts_and_tokens_until_their_expiry(pool: PgPool) {
    set_operator(&pool).await;
    let now = start();
    insert_attempt(&pool, now - TimeDelta::days(90)).await;
    insert_attempt(&pool, now - TimeDelta::days(89)).await;
    insert_token(&pool, 1, now).await;
    insert_token(&pool, 2, now + TimeDelta::seconds(1)).await;

    let pruned = access::prune(&pool, now).await.unwrap();
    assert_eq!(
        pruned,
        access::Pruned {
            attempts: 1,
            tokens: 1
        }
    );
    let attempt_at: DateTime<Utc> = sqlx::query_scalar("select at from access.login_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempt_at, now - TimeDelta::days(89));
    let token_expiry: DateTime<Utc> =
        sqlx::query_scalar("select expires_at from access.api_tokens")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(token_expiry, now + TimeDelta::seconds(1));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_9_the_database_refuses_a_username_outside_the_pattern(pool: PgPool) {
    let insert = sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values (gen_random_uuid(), 'an operator', 'x', now())",
    )
    .execute(&pool)
    .await;
    let code = insert
        .unwrap_err()
        .as_database_error()
        .and_then(|e| e.code().map(std::borrow::Cow::into_owned));
    assert_eq!(code.as_deref(), Some("23514"), "check_violation");
    let long = count(&pool, "select count(*) from access.operators");
    assert_eq!(long.await, 0);
}
