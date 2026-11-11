//! MET-017, K8 Section 6: many requests at once. One node's requests beyond two wait for no
//! lookup, so its junk keeps no agent out; a request waits in order for a database turn; and a
//! body that never ends holds no database turn.

use std::time::Duration;

use agent_core::wire::{AgentToken, EnrolmentCode};
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::Instant;

use super::{
    AGENT_ROW, address, at_once, batch, device, hold_row, hub_pool, post, renewed_pve, router,
    router_within, send, spawn_push, unavailable, within,
};
use crate::metrics::ingest::{
    tests::{database_now, enrolled, enrolled_pve, enrolment::waits_for_locks, pve},
    turns::{ADDRESS_REQUESTS, AGENT_REQUESTS, REQUEST_WAIT},
};

/// While two requests of one address wait for the database, its third is `UNAVAILABLE` at once
/// and another address is served.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_an_address_has_two_requests_in_flight_until_their_tokens_are_known(pool: PgPool) {
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
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
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
    tokio::time::sleep(REQUEST_WAIT / 4).await;
    assert!(!queued.is_finished(), "sent back while it could still wait");
    holder.rollback().await.unwrap();
    let answer = within(REQUEST_WAIT, async { queued.await.unwrap() }).await;
    assert_eq!(answer.0, StatusCode::NO_CONTENT, "{answer:?}");
    for push in held {
        push.await.unwrap();
    }
}

/// T8a review probe P1 as a test: one node floods junk bearers and enrolment codes while an
/// honest agent on another node pushes; every honest push is stored.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_a_flood_from_one_node_without_credentials_keeps_no_agent_out(pool: PgPool) {
    let now = database_now(&pool).await;
    let token = enrolled_pve(&pool, now, 1).await;
    let router = router(&hub_pool(&pool).await, now);
    let until = Instant::now() + Duration::from_secs(2);
    let attackers: Vec<_> = (0..64_u8)
        .map(|n| {
            let router = router.clone();
            tokio::spawn(async move {
                while Instant::now() < until {
                    let flooding = ("/agent/v1/push", address(66));
                    let junk = AgentToken::from_random([n.wrapping_add(100); 32]);
                    let bearer = Some(junk.expose().to_owned());
                    post(router.clone(), flooding, bearer, Body::from("{}")).await;
                    let code = EnrolmentCode::from_random([n; 16]);
                    let body = json!({ "code": code.expose() }).to_string();
                    let enrolling = ("/agent/v1/enrol", address(66));
                    post(router.clone(), enrolling, None, Body::from(body)).await;
                    // A refusal needs no I/O; across a network the next request arrives later.
                    tokio::task::yield_now().await;
                }
            })
        })
        .collect();
    let mut honest = Vec::new();
    while Instant::now() < until {
        honest.push(
            send(router.clone(), address(7), &token, batch(pve(), now))
                .await
                .0,
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    for attacker in attackers {
        attacker.await.unwrap();
    }
    assert!(honest.len() >= 5, "{honest:?}");
    assert!(
        honest
            .iter()
            .all(|status| *status == StatusCode::NO_CONTENT),
        "{honest:?}"
    );
}

/// T8a review probe P2 as a test: four agents whose bodies never end hold only their own
/// device's turn, so a fifth is served at once; each slow body is `UNAVAILABLE` after its body
/// deadline, well before the answer deadline.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_017_bodies_that_never_end_hold_no_request_turn(pool: PgPool) {
    let now = database_now(&pool).await;
    let mut tokens = Vec::new();
    for n in 1..=5_u8 {
        tokens.push(enrolled(&pool, device(n), now, n).await);
    }
    let router = router_within(&hub_pool(&pool).await, now, Duration::from_secs(30));
    let slow: Vec<_> = tokens[..4]
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
        .collect();
    tokio::time::sleep(Duration::from_millis(300)).await;
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
        let answer = within(Duration::from_secs(8), async { push.await.unwrap() }).await;
        assert_eq!(answer, unavailable());
    }
}
