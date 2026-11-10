//! The bearer layer judges a token at the hub's clock, and a lookup that fails for any reason
//! but an unreachable database gets an empty 500 (A-9, K3, K7).

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anima_hub::{
    access::{self, Scope},
    clock::{Clock, FixedClock},
};
use axum::{Router, http::StatusCode, routing::get};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{READ_KEY, WRITE_KEY, assert_refused, database_now, scrape_with, stored};

/// `/metrics` behind a READ bearer at `now`; counts the requests that reached the route.
fn guarded(pool: &PgPool, now: DateTime<Utc>) -> (Router, Arc<AtomicUsize>) {
    let reached = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&reached);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let route = get(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        async { "reached" }
    });
    let router = Router::new()
        .route("/metrics", route)
        .route_layer(access::bearer_layer(pool.clone(), clock, Scope::Read));
    (router, reached)
}

#[sqlx::test(migrations = "./migrations")]
async fn a_9_a_token_expires_by_the_hubs_clock_not_the_machines(pool: PgPool) {
    let now = database_now(&pool).await;
    // Unexpired by the machine's time, expired by a hub clock three hours ahead.
    let lapsed = stored(&pool, READ_KEY, now + TimeDelta::hours(2)).await;
    // Expired by the machine's time, unexpired by a hub clock three hours behind.
    let early = stored(&pool, WRITE_KEY, now - TimeDelta::hours(2)).await;
    let (ahead, _) = guarded(&pool, now + TimeDelta::hours(3));
    let (behind, _) = guarded(&pool, now - TimeDelta::hours(3));

    assert_refused(&scrape_with(&ahead, &lapsed).await, "ahead, lapsed");
    assert_refused(&scrape_with(&ahead, &early).await, "ahead, early");
    assert_eq!(scrape_with(&behind, &lapsed).await.status, StatusCode::OK);
    assert_eq!(scrape_with(&behind, &early).await.status, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_9_a_lookup_fault_gets_an_empty_500_and_no_route(pool: PgPool) {
    let now = database_now(&pool).await;
    let key = stored(&pool, READ_KEY, now + TimeDelta::days(1)).await;
    let (app, reached) = guarded(&pool, now);
    // The control: the key passes while its table is there.
    assert_eq!(scrape_with(&app, &key).await.status, StatusCode::OK);
    sqlx::query("ALTER TABLE access.api_tokens RENAME TO api_tokens_moved")
        .execute(&pool)
        .await
        .unwrap();

    let answer = scrape_with(&app, &key).await;

    assert_eq!(
        answer.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{answer:?}"
    );
    assert_eq!(answer.body, "");
    assert!(!answer.challenge);
    assert_eq!(reached.load(Ordering::SeqCst), 1);
}
