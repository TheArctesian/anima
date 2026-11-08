//! A second `signInBrowser` whose request still carries a browser's cookie deletes the token that
//! cookie held, so no browser token outlives the cookie that held it; a refused or failed sign-in
//! deletes nothing (SEC-002, D-3, A-3, A-4).

use std::sync::Arc;

use anima_hub::clock::{Clock, FixedClock};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    Answer, PUBLIC, browser_hub, browser_token, cookie, hash, me, sign_in_with,
    socket::{FORBIDDEN, ack, init, open, receive, serve_watched},
};
use crate::hub::{Hub, PASSWORD, USERNAME, error, set_operator, start, wrong_credentials};

/// The headers of a request from the public origin with `cookie`.
fn from_public(cookie: &str) -> [(&'static str, &str); 2] {
    [("origin", PUBLIC), ("cookie", cookie)]
}

/// Whether `token` still has its row.
async fn stored(pool: &PgPool, token: &str) -> bool {
    sqlx::query_scalar("select exists (select from access.api_tokens where token_hash = $1)")
        .bind(hash(token))
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The audit actions, sorted.
async fn audit_actions(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar("select action from events.audit_log order by action")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The token the signed-in `answer` set.
fn signed_in(answer: &Answer) -> String {
    assert!(
        answer.json()["data"]["signInBrowser"].is_object(),
        "{}",
        answer.body
    );
    answer.token()
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_second_sign_in_deletes_the_token_its_cookie_held(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let first = cookie(&browser_token(&hub).await);

    let second = signed_in(&sign_in_with(&hub, &from_public(&first), PASSWORD).await);

    let first_token = first.strip_prefix("__Host-anima=").unwrap();
    assert!(!stored(&pool, first_token).await, "the first row is gone");
    assert!(stored(&pool, &second).await);
    assert_eq!(
        me(&hub, &from_public(&first)).await,
        Err("UNAUTHENTICATED".to_owned())
    );
    assert_eq!(
        me(&hub, &from_public(&cookie(&second))).await,
        Ok(USERNAME.to_owned())
    );
    assert_eq!(audit_actions(&pool).await, ["login", "login", "logout"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_failed_or_throttled_second_sign_in_keeps_the_first_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let first = cookie(&browser_token(&hub).await);

    for _ in 0..5 {
        let wrong = sign_in_with(&hub, &from_public(&first), "wrong password").await;
        assert_eq!(error(&wrong.json()), wrong_credentials());
        assert_eq!(
            me(&hub, &from_public(&first)).await,
            Ok(USERNAME.to_owned())
        );
    }
    let throttled = sign_in_with(&hub, &from_public(&first), PASSWORD).await;
    assert_eq!(error(&throttled.json()), wrong_credentials(), "throttled");

    assert_eq!(
        me(&hub, &from_public(&first)).await,
        Ok(USERNAME.to_owned())
    );
    assert_eq!(audit_actions(&pool).await, ["login"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_4_a_cross_origin_second_sign_in_is_refused_and_keeps_the_first_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let first = cookie(&browser_token(&hub).await);

    for origin in ["https://evil.example", "https://dev.net.anima.ing"] {
        let answer = sign_in_with(&hub, &[("origin", origin), ("cookie", &first)], PASSWORD).await;
        assert_eq!(answer.json()["data"], Value::Null, "{origin}");
        assert_eq!(error(&answer.json()).1, "FORBIDDEN", "{origin}");
    }

    assert_eq!(
        me(&hub, &from_public(&first)).await,
        Ok(USERNAME.to_owned())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_3_a_sign_in_under_a_bearer_replaces_neither_token(pool: PgPool) {
    set_operator(&pool).await;
    let login = Hub::new(&pool, start()).token().await;
    let hub = browser_hub(&pool, start());
    let first = cookie(&browser_token(&hub).await);
    let bearer = format!("Bearer {login}");

    let headers = [
        ("origin", PUBLIC),
        ("cookie", &first),
        ("authorization", &bearer),
    ];
    signed_in(&sign_in_with(&hub, &headers, PASSWORD).await);

    assert!(stored(&pool, &login).await, "the login token stays");
    assert_eq!(
        me(&hub, &from_public(&first)).await,
        Ok(USERNAME.to_owned())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_socket_on_the_replaced_token_closes_at_the_next_recheck(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let first = cookie(&browser_token(&hub).await);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let served = serve_watched(&pool, clock).await;
    let mut socket = open(served.addr, &from_public(&first)).await;
    assert_eq!(init(&mut socket, Value::Null).await, Ok(ack()));

    let second = signed_in(&sign_in_with(&hub, &from_public(&first), PASSWORD).await);

    assert_eq!(receive(&mut socket).await, FORBIDDEN);
    let mut fresh = open(served.addr, &from_public(&cookie(&second))).await;
    assert_eq!(init(&mut fresh, json!({})).await, Ok(ack()));
}
