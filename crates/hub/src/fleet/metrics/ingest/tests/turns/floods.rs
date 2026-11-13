//! MET-017, K8 Section 6: many requests at once. One node's requests beyond two wait for no
//! lookup, so its junk keeps no agent out; a request waits in order for a database turn; and a
//! body that never ends holds no database turn.

use std::time::Duration;

use agent_core::wire::{AgentToken, EnrolmentCode};
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{
    AGENT_ROW, PATIENT, SHORT, address, at_once, batch, device, hold_row, hub_pool, post,
    renewed_pve, router, router_within, send, spawn_push, unavailable, within,
};
use crate::fleet::metrics::ingest::{
    routes::Deadlines,
    tests::{database_now, enrolled, enrolled_pve, enrolment::waits_for_locks, pve},
    turns::{ADDRESS_REQUESTS, AGENT_REQUESTS},
};

/// T8a review probe P1, held still: while two requests of node 66 wait for the database, its
/// next ones, junk bearer, junk code or a valid token, are `UNAVAILABLE` at once with no lookup,
/// and node 7 is served.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_a_node_whose_two_turns_are_taken_is_refused_at_once_and_others_are_served(
    pool: PgPool,
) {
    let now = database_now(&pool).await;
    let presented = renewed_pve(&pool, now).await;
    let neighbour = enrolled(&pool, device(2), now, 2).await;
    let router = router(&hub_pool(&pool).await, now);
    let holder = hold_row(&pool, AGENT_ROW, pve()).await;
    let waiting: Vec<_> = (0..ADDRESS_REQUESTS)
        .map(|_| spawn_push(&router, address(66), &presented, batch(pve(), now)))
        .collect();
    let last = waiting.last().unwrap();
    assert!(
        waits_for_locks(&pool, last, 2).await,
        "both wait for the agent row"
    );

    let junk = AgentToken::from_random([100; 32]);
    let bearer = Some(junk.expose().to_owned());
    let pushing = ("/agent/v1/push", address(66));
    let junk_push = post(router.clone(), pushing, bearer, Body::from("{}"));
    assert_eq!(at_once(junk_push).await, unavailable());
    let code = EnrolmentCode::from_random([7; 16]);
    let body = Body::from(json!({ "code": code.expose() }).to_string());
    let enrolling = ("/agent/v1/enrol", address(66));
    let junk_enrol = post(router.clone(), enrolling, None, body);
    assert_eq!(at_once(junk_enrol).await, unavailable());
    let third = send(
        router.clone(),
        address(66),
        &neighbour,
        batch(device(2), now),
    );
    assert_eq!(at_once(third).await, unavailable());

    let other = send(
        router.clone(),
        address(7),
        &neighbour,
        batch(device(2), now),
    );
    assert_eq!(at_once(other).await.0, StatusCode::NO_CONTENT);
    holder.rollback().await.unwrap();
    for push in waiting {
        push.await.unwrap();
    }
}

/// While every request turn is taken, a request waits for one instead of being sent back, and
/// is served once a turn ends.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_a_request_waits_for_a_turn_while_all_are_taken(pool: PgPool) {
    let now = database_now(&pool).await;
    let presented = renewed_pve(&pool, now).await;
    let neighbour = enrolled(&pool, device(2), now, 2).await;
    let router = router(&hub_pool(&pool).await, now);
    let holder = hold_row(&pool, AGENT_ROW, pve()).await;
    let held: Vec<_> = (0..AGENT_REQUESTS)
        .map(|n| {
            spawn_push(
                &router,
                address(20 + u8::try_from(n).unwrap()),
                &presented,
                batch(pve(), now),
            )
        })
        .collect();
    let count = i64::try_from(AGENT_REQUESTS).unwrap();
    assert!(waits_for_locks(&pool, held.last().unwrap(), count).await);

    let queued = spawn_push(&router, address(7), &neighbour, batch(device(2), now));
    tokio::time::sleep(SHORT).await;
    assert!(!queued.is_finished(), "sent back while it could still wait");
    holder.rollback().await.unwrap();
    let answer = within(Duration::from_secs(10), async { queued.await.unwrap() }).await;
    assert_eq!(answer.0, StatusCode::NO_CONTENT, "{answer:?}");
    for push in held {
        push.await.unwrap();
    }
}

/// Pushes from devices 1 to 4 at addresses 30 to 33 whose bodies never end.
fn endless_pushes(
    router: &axum::Router,
    tokens: &[AgentToken],
) -> Vec<tokio::task::JoinHandle<(StatusCode, String)>> {
    tokens
        .iter()
        .enumerate()
        .map(|(n, token)| {
            let route = ("/agent/v1/push", address(30 + u8::try_from(n).unwrap()));
            let bearer = Some(token.expose().to_owned());
            let never = futures_util::stream::pending::<Result<Bytes, std::io::Error>>();
            tokio::spawn(post(
                router.clone(),
                route,
                bearer,
                Body::from_stream(never),
            ))
        })
        .collect()
}

/// T8a review probe P2: four agents whose bodies never end hold only their own device's turn,
/// so a fifth is served at once, long before their body deadline.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_bodies_that_never_end_hold_no_request_turn(pool: PgPool) {
    let now = database_now(&pool).await;
    let mut tokens = Vec::new();
    for n in 1..=5_u8 {
        tokens.push(enrolled(&pool, device(n), now, n).await);
    }
    let router = router(&hub_pool(&pool).await, now);
    let slow = endless_pushes(&router, &tokens[..4]);
    tokio::time::sleep(SHORT).await;
    for _ in 0..3 {
        let honest = send(
            router.clone(),
            address(7),
            &tokens[4],
            batch(device(5), now),
        );
        assert_eq!(at_once(honest).await.0, StatusCode::NO_CONTENT);
    }
    for push in slow {
        assert!(!push.is_finished(), "answered before its body deadline");
        push.abort();
    }
}

/// A body that has not arrived by its deadline is `UNAVAILABLE`, well before the answer's.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_a_body_that_never_ends_is_unavailable_at_its_deadline(pool: PgPool) {
    let now = database_now(&pool).await;
    let token = enrolled(&pool, device(1), now, 1).await;
    let deadlines = Deadlines {
        body: SHORT,
        ..PATIENT
    };
    let router = router_within(&hub_pool(&pool).await, now, deadlines);
    for push in endless_pushes(&router, &[token]) {
        let answer = within(Duration::from_secs(10), async { push.await.unwrap() }).await;
        assert_eq!(answer, unavailable());
    }
}

/// T8a review probe R3 N1 held still: while the event log is locked, four enrolments with junk
/// codes from four nodes wait to write their refusal events. They hold no database turn while
/// they wait, so an honest push from a fifth node is stored at once.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_refusal_events_that_wait_hold_no_request_turn(pool: PgPool) {
    let now = database_now(&pool).await;
    let token = enrolled_pve(&pool, now, 1).await;
    let router = router(&hub_pool(&pool).await, now);
    let first = send(router.clone(), address(7), &token, batch(pve(), now)).await;
    assert_eq!(
        first.0,
        StatusCode::NO_CONTENT,
        "a first push writes its event now"
    );
    let mut log = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE events.event_log IN EXCLUSIVE MODE")
        .execute(&mut *log)
        .await
        .unwrap();

    let junk: Vec<_> = (0..AGENT_REQUESTS)
        .map(|n| {
            let n = u8::try_from(n).unwrap();
            let code = EnrolmentCode::from_random([200 + n; 16]);
            let body = Body::from(json!({ "code": code.expose() }).to_string());
            tokio::spawn(post(
                router.clone(),
                ("/agent/v1/enrol", address(40 + n)),
                None,
                body,
            ))
        })
        .collect();
    let count = i64::try_from(AGENT_REQUESTS).unwrap();
    assert!(
        waits_for_locks(&pool, junk.last().unwrap(), count).await,
        "each refusal waits to write its event"
    );

    let later = batch(pve(), now + TimeDelta::seconds(10));
    let honest = send(router.clone(), address(8), &token, later);
    assert_eq!(at_once(honest).await.0, StatusCode::NO_CONTENT);
    log.rollback().await.unwrap();
    for enrolment in junk {
        assert_eq!(enrolment.await.unwrap().0, StatusCode::UNAUTHORIZED);
    }
}
