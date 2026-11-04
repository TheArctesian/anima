//! Signing in on the `/graphql` WebSocket of the hub's router: a Bearer `connection_init`
//! payload gets events; anything else closes with 4403 (K7 "Authentication transport", R-020,
//! API-002, H-6(b)).

use std::sync::Arc;

use anima_hub::{
    access::MESSAGE_LIMIT,
    clock::{Clock, FixedClock},
    events::Severity,
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_tungstenite::tungstenite;

use super::client::{
    Received, ack, bearer, connect, init, next_event, open, publish, receive, send, serve_router,
    subscribe_events,
};
use crate::hub::{Hub, set_operator, start};

const FORBIDDEN: Received = Received::Closed(Some(4403));

async fn signed_up(pool: &PgPool) -> String {
    set_operator(pool).await;
    Hub::new(pool, start()).token().await
}

fn at_start() -> Arc<dyn Clock> {
    Arc::new(FixedClock(start()))
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_log_010_a_bearer_connection_init_gets_events_at_or_above_min_severity(pool: PgPool) {
    let token = signed_up(&pool).await;
    let served = serve_router(&pool, at_start()).await;
    let mut socket = open(served.addr).await;
    subscribe_events(&mut socket, &served.bus, &token, "WARNING").await;
    publish(&served.bus, Severity::Info, "probe ok");
    publish(&served.bus, Severity::Warning, "disk 91%");
    publish(&served.bus, Severity::Error, "router lost");
    for (severity, message) in [("WARNING", "disk 91%"), ("ERROR", "router lost")] {
        let event = json!({ "severity": severity, "message": message });
        assert_eq!(next_event(&mut socket).await, Received::Json(event));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_a_missing_or_invalid_token_closes_with_4403(pool: PgPool) {
    let token = signed_up(&pool).await;
    let served = serve_router(&pool, at_start()).await;
    for payload in [
        Value::Null,
        json!({}),
        json!({ "authorization": null }),
        json!({ "authorization": token }),
        json!({ "authorization": format!("Basic {token}") }),
        json!({ "authorization": format!("Bearer {token}x") }),
        json!({ "authorization": "Bearer " }),
        json!({ "token": token }),
        json!({ "Authorization": format!("Bearer {token}") }),
        json!({ "authorization": { "Bearer": token } }),
        json!(format!("Bearer {token}")),
    ] {
        let mut socket = open(served.addr).await;
        assert_eq!(
            init(&mut socket, payload.clone()).await,
            FORBIDDEN,
            "{payload}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_an_expired_token_closes_with_4403(pool: PgPool) {
    let token = signed_up(&pool).await;
    let expired: Arc<dyn Clock> = Arc::new(FixedClock(start() + TimeDelta::days(30)));
    let served = serve_router(&pool, expired).await;
    let mut socket = open(served.addr).await;
    assert_eq!(init(&mut socket, bearer(&token)).await, FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_a_token_in_the_url_is_never_read(pool: PgPool) {
    let token = signed_up(&pool).await;
    let served = serve_router(&pool, at_start()).await;
    for path in [
        format!("/graphql?token={token}"),
        format!("/graphql?authorization=Bearer%20{token}"),
    ] {
        let mut socket = connect(served.addr, &path, "graphql-transport-ws")
            .await
            .unwrap();
        assert_eq!(init(&mut socket, json!({})).await, FORBIDDEN, "{path}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn api_002_only_the_graphql_transport_ws_subprotocol_is_served(pool: PgPool) {
    let served = serve_router(&pool, at_start()).await;
    for offered in ["graphql-ws", "chat"] {
        match connect(served.addr, "/graphql", offered).await {
            Err(tungstenite::Error::Http(response)) => {
                assert_eq!(response.status(), 400, "{offered}");
            }
            other => panic!("{offered}: {:?}", other.map(|_| "connected")),
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn r_020_subscribe_before_sign_in_and_a_second_init_are_refused(pool: PgPool) {
    let token = signed_up(&pool).await;
    let served = serve_router(&pool, at_start()).await;
    let mut early = open(served.addr).await;
    let subscribe = json!({ "id": "1", "type": "subscribe",
        "payload": { "query": "subscription { eventAdded { message } }" } });
    send(&mut early, &subscribe).await;
    assert_eq!(receive(&mut early).await, Received::Closed(Some(4401)));

    let mut twice = open(served.addr).await;
    assert_eq!(init(&mut twice, bearer(&token)).await, ack());
    assert_eq!(
        init(&mut twice, bearer(&token)).await,
        Received::Closed(Some(4429))
    );

    let mut garbled = open(served.addr).await;
    send(&mut garbled, &json!({ "type": "nonsense" })).await;
    assert_eq!(receive(&mut garbled).await, Received::Closed(Some(4400)));
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_mutation_over_the_socket_is_audited_with_the_peer_address(pool: PgPool) {
    let token = signed_up(&pool).await;
    let served = serve_router(&pool, at_start()).await;
    let mut socket = open(served.addr).await;
    assert_eq!(init(&mut socket, bearer(&token)).await, ack());
    let logout =
        json!({ "id": "1", "type": "subscribe", "payload": { "query": "mutation { logout }" } });
    send(&mut socket, &logout).await;
    let next = json!({ "id": "1", "type": "next", "payload": { "data": { "logout": true } } });
    assert_eq!(receive(&mut socket).await, Received::Json(next));
    let clients: Vec<String> = sqlx::query_scalar(
        "select host(client_addr) from events.audit_log where action = 'logout'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(clients, ["127.0.0.1"]);
}

/// A `connection_init` with a valid token, padded to `bytes`.
fn padded_init(token: &str, bytes: usize) -> Value {
    let mut payload = bearer(token);
    payload["padding"] = json!("x".repeat(bytes));
    json!({ "type": "connection_init", "payload": payload })
}

#[sqlx::test(migrations = "./migrations")]
async fn h_6_a_message_over_the_limit_closes_the_socket_before_sign_in(pool: PgPool) {
    let token = signed_up(&pool).await;
    let served = serve_router(&pool, at_start()).await;
    let mut within = open(served.addr).await;
    send(&mut within, &padded_init(&token, MESSAGE_LIMIT - 1024)).await;
    assert_eq!(receive(&mut within).await, ack());

    let mut over = open(served.addr).await;
    send(&mut over, &padded_init(&token, MESSAGE_LIMIT)).await;
    let answer = receive(&mut over).await;
    assert!(
        matches!(answer, Received::Closed(None | Some(1008 | 1009))),
        "{answer:?}"
    );
}
