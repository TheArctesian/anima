//! The hub's runtime role can run the sign-in flow on the access tables (R-016, K2 "Roles").

use std::net::{IpAddr, Ipv4Addr};

use chrono::TimeDelta;
use sqlx::PgPool;

use super::now;
use crate::access::{
    store::{self, AttemptKey, NewToken, Outcome},
    tokens::{Scope, Token},
};

/// STAND-IN until WP-05 T1's migration `0001_access_runtime_role.sql` is on this branch: it
/// creates `anima_runtime` and the owner's default privileges. Remove this function and its
/// call at that merge; the test must then pass on the default privileges alone.
async fn stand_in_runtime_role(pool: &PgPool) {
    let statements = [
        "do $$ begin \
           if not exists (select from pg_catalog.pg_roles where rolname = 'anima_runtime') then \
             begin create role anima_runtime nologin; \
             exception when duplicate_object or unique_violation then null; end; \
           end if; \
         end $$",
        "grant usage on schema access to anima_runtime",
        "grant select, insert, update, delete on all tables in schema access to anima_runtime",
    ];
    for sql in statements {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_006_r_016_runtime_role_runs_the_login_flow_on_access_tables(pool: PgPool) {
    stand_in_runtime_role(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("set role anima_runtime")
        .execute(&mut *conn)
        .await
        .unwrap();

    let (id, _) = store::replace_operator(&mut conn, "operator", "$argon2id$x", now())
        .await
        .unwrap();
    let token = Token::from_random([3; 32]);
    let new_token = NewToken {
        operator: id,
        hash: token.hash(),
        scope: Scope::Write,
        expires_at: now() + TimeDelta::days(30),
    };
    let token_id = store::insert_token(&mut conn, &new_token, now())
        .await
        .unwrap();
    let client = IpAddr::V4(Ipv4Addr::new(100, 64, 0, 7));
    let attempt = store::start_attempt(&mut *conn, now(), "operator", client)
        .await
        .unwrap();
    let key = AttemptKey {
        id: attempt,
        at: now(),
        username: "operator",
        client,
    };
    store::failures(&mut conn, &key, now() - TimeDelta::minutes(30))
        .await
        .unwrap();
    store::finish_attempt(&mut *conn, attempt, Outcome::Ok)
        .await
        .unwrap();
    let account = store::account_by_username(&mut *conn, "operator")
        .await
        .unwrap();
    assert_eq!(account.map(|a| a.id), Some(id));
    let stored = store::token_by_hash(&mut *conn, token.hash())
        .await
        .unwrap();
    assert_eq!(stored.map(|t| t.id), Some(token_id));
    store::touch_token(&mut *conn, token_id, now())
        .await
        .unwrap();
    assert!(store::delete_token(&mut conn, token_id).await.unwrap());
    store::delete_operator_tokens(&mut conn, id).await.unwrap();
    store::delete_attempts_of(&mut conn, &["operator"])
        .await
        .unwrap();
    store::prune(&mut conn, now(), now()).await.unwrap();

    sqlx::query("reset role").execute(&mut *conn).await.unwrap();
}
