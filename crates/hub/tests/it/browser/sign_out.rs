//! `logout` over `POST /graphql` closes every socket signed in with the same token at its next
//! re-check, for a browser cookie and for a desktop login token (A-8, SEC-002, K7).

use std::sync::Arc;

use anima_hub::clock::{Clock, FixedClock};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    PUBLIC, browser_hub, browser_token, cookie, post,
    socket::{FORBIDDEN, ack, init, open, receive, serve_watched},
};
use crate::hub::{Hub, set_operator, start};

const LOGOUT: &str = "mutation { logout }";

#[sqlx::test(migrations = "./migrations")]
async fn a_8_logout_closes_a_cookie_socket_on_the_same_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let cookie = cookie(&browser_token(&hub).await);
    let headers = [("origin", PUBLIC), ("cookie", cookie.as_str())];
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let served = serve_watched(&pool, clock).await;
    let mut socket = open(served.addr, &headers).await;
    assert_eq!(init(&mut socket, Value::Null).await, Ok(ack()));

    let answer = post(&hub, &headers, LOGOUT, json!({})).await;

    assert_eq!(answer.json(), json!({ "data": { "logout": true } }));
    assert_eq!(receive(&mut socket).await, FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_logout_closes_a_socket_on_the_same_login_token(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let bearer = format!("Bearer {token}");
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let served = serve_watched(&pool, clock).await;
    let mut socket = open(served.addr, &[]).await;
    let payload = json!({ "authorization": bearer });
    assert_eq!(init(&mut socket, payload).await, Ok(ack()));

    let hub = browser_hub(&pool, start());
    let answer = post(&hub, &[("authorization", &bearer)], LOGOUT, json!({})).await;

    assert_eq!(answer.json(), json!({ "data": { "logout": true } }));
    assert_eq!(receive(&mut socket).await, FORBIDDEN);
}
