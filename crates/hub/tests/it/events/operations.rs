//! How many operations one signed-in socket may run: at most 32 at once, and each id once
//! (H-6(b), API-004).

use std::sync::Arc;

use anima_hub::clock::FixedClock;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::client::{
    Received, Served, Socket, ack, bearer, init, open, receive, send, serve_router,
};
use crate::hub::{Hub, set_operator, start};

/// H-6(b): written out so that raising the hub's cap fails these tests.
const CAP: usize = 32;

fn subscribe_events(id: &str) -> Value {
    json!({ "id": id, "type": "subscribe",
        "payload": { "query": "subscription { eventAdded { message } }" } })
}

/// Sends a ping and returns the answer: a pong while the connection serves.
async fn ping(socket: &mut Socket) -> Received {
    send(socket, &json!({ "type": "ping" })).await;
    receive(socket).await
}

fn pong() -> Received {
    Received::Json(json!({ "type": "pong" }))
}

async fn signed_in(pool: &PgPool) -> (Served, Socket) {
    set_operator(pool).await;
    let token = Hub::new(pool, start()).token().await;
    let served = serve_router(pool, Arc::new(FixedClock(start()))).await;
    let mut socket = open(served.addr).await;
    assert_eq!(init(&mut socket, bearer(&token)).await, ack());
    (served, socket)
}

#[sqlx::test(migrations = "./migrations")]
async fn h_6_a_duplicate_operation_id_closes_with_4409(pool: PgPool) {
    let (_served, mut socket) = signed_in(&pool).await;
    send(&mut socket, &subscribe_events("1")).await;
    assert_eq!(ping(&mut socket).await, pong());
    send(&mut socket, &subscribe_events("1")).await;
    assert_eq!(receive(&mut socket).await, Received::Closed(Some(4409)));
}

#[sqlx::test(migrations = "./migrations")]
async fn h_6_the_33rd_running_operation_closes_the_socket(pool: PgPool) {
    let (_served, mut socket) = signed_in(&pool).await;
    for id in 0..CAP {
        send(&mut socket, &subscribe_events(&id.to_string())).await;
    }
    assert_eq!(ping(&mut socket).await, pong());
    let complete = json!({ "id": "0", "type": "complete" });
    send(&mut socket, &complete).await;
    assert_eq!(receive(&mut socket).await, Received::Json(complete));
    send(&mut socket, &subscribe_events("again")).await;
    assert_eq!(ping(&mut socket).await, pong());
    send(&mut socket, &subscribe_events("one-more")).await;
    assert_eq!(receive(&mut socket).await, Received::Closed(Some(1008)));
}

#[sqlx::test(migrations = "./migrations")]
async fn h_6_an_operation_the_hub_completes_no_longer_counts(pool: PgPool) {
    let (_served, mut socket) = signed_in(&pool).await;
    for id in 0..CAP - 1 {
        send(&mut socket, &subscribe_events(&id.to_string())).await;
    }
    for query in 0..3 {
        let id = format!("q{query}");
        let typename = json!({ "id": id, "type": "subscribe",
            "payload": { "query": "{ __typename }" } });
        send(&mut socket, &typename).await;
        let next = json!({ "id": id, "type": "next",
            "payload": { "data": { "__typename": "Query" } } });
        assert_eq!(receive(&mut socket).await, Received::Json(next));
        let complete = json!({ "id": id, "type": "complete" });
        assert_eq!(receive(&mut socket).await, Received::Json(complete));
    }
    send(&mut socket, &subscribe_events("last")).await;
    assert_eq!(ping(&mut socket).await, pong());
    send(&mut socket, &subscribe_events("one-more")).await;
    assert_eq!(receive(&mut socket).await, Received::Closed(Some(1008)));
}
