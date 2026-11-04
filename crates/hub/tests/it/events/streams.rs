//! An open subscription ends when its token does: at expiry by the hub clock, after logout or
//! `operator set`, and when the token cannot be checked (R-020, API-003, D-6).

use std::{sync::Arc, time::Duration};

use anima_hub::{
    access::{self, Password},
    clock::Clock,
    events::Severity,
};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::client::{
    Received, TestClock, next_event, open, publish, receive, serve_watched, subscribe_events,
};
use crate::hub::{Hub, PASSWORD, USERNAME, set_operator, start};

const RECHECK: Duration = Duration::from_millis(50);
const INIT_TIMEOUT: Duration = Duration::from_secs(10);
const FORBIDDEN: Received = Received::Closed(Some(4403));

#[sqlx::test(migrations = "./migrations")]
async fn r_020_the_stream_ends_at_token_expiry_by_the_hub_clock(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let clock = TestClock::at(start());
    let shared: Arc<dyn Clock> = Arc::new(clock.clone());
    let served = serve_watched(&pool, shared, RECHECK, INIT_TIMEOUT).await;
    let mut socket = open(served.addr).await;
    subscribe_events(&mut socket, &served.bus, &token, "INFO").await;

    // Several re-checks pass while the token holds.
    clock.set(start() + TimeDelta::days(30) - TimeDelta::seconds(1));
    tokio::time::sleep(RECHECK * 4).await;
    publish(&served.bus, Severity::Info, "still signed in");
    let event = json!({ "severity": "INFO", "message": "still signed in" });
    assert_eq!(next_event(&mut socket).await, Received::Json(event));

    clock.set(start() + TimeDelta::days(30));
    assert_eq!(next_event(&mut socket).await, FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_logout_ends_the_stream_of_that_token_only(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let (leaving, staying) = (hub.token().await, hub.token().await);
    let clock: Arc<dyn Clock> = Arc::new(TestClock::at(start()));
    let served = serve_watched(&pool, clock, RECHECK, INIT_TIMEOUT).await;
    let mut gone = open(served.addr).await;
    subscribe_events(&mut gone, &served.bus, &leaving, "INFO").await;
    let mut kept = open(served.addr).await;
    subscribe_events(&mut kept, &served.bus, &staying, "INFO").await;

    let logout = hub
        .post(Some(&leaving), "mutation { logout }", json!({}))
        .await;
    assert_eq!(logout["data"]["logout"], true, "{logout}");
    assert_eq!(next_event(&mut gone).await, FORBIDDEN);

    tokio::time::sleep(RECHECK * 4).await;
    publish(&served.bus, Severity::Info, "after logout");
    let event = json!({ "severity": "INFO", "message": "after logout" });
    assert_eq!(next_event(&mut kept).await, Received::Json(event));
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_operator_set_ends_every_stream(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let clock: Arc<dyn Clock> = Arc::new(TestClock::at(start()));
    let served = serve_watched(&pool, clock, RECHECK, INIT_TIMEOUT).await;
    let mut socket = open(served.addr).await;
    subscribe_events(&mut socket, &served.bus, &token, "INFO").await;

    let password = Password::new(PASSWORD.to_owned());
    access::set_operator(&pool, USERNAME, password, start())
        .await
        .unwrap();
    assert_eq!(next_event(&mut socket).await, FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_a_token_that_cannot_be_checked_ends_the_stream(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let clock: Arc<dyn Clock> = Arc::new(TestClock::at(start()));
    let served = serve_watched(&pool, clock, RECHECK, INIT_TIMEOUT).await;
    let mut socket = open(served.addr).await;
    subscribe_events(&mut socket, &served.bus, &token, "INFO").await;

    pool.close().await;
    assert_eq!(next_event(&mut socket).await, Received::Closed(Some(1011)));
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_a_client_that_never_signs_in_is_closed_with_4408(pool: PgPool) {
    let clock: Arc<dyn Clock> = Arc::new(TestClock::at(start()));
    let served = serve_watched(&pool, clock, RECHECK, Duration::from_millis(200)).await;
    let mut socket = open(served.addr).await;
    assert_eq!(receive(&mut socket).await, Received::Closed(Some(4408)));
}
