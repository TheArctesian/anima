//! The D-23 kind rule and the read rule over a WebSocket signed in with an API key through
//! `connection_init`: the same refusals as over HTTP, each a `next` frame with the error and
//! then `complete`, and nothing written (D-23, R-100, API-003).

use std::sync::Arc;

use anima_hub::clock::{Clock, FixedClock};
use async_graphql::futures_util::SinkExt as _;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio_tungstenite::tungstenite::Message;

use super::{count, kind_refused, secret, signed_in};
use crate::{
    browser::socket::{Socket, ack, init, open, receive, serve_watched},
    hub::{USERNAME, error, start},
};

/// Runs `query` as operation `id`; returns every frame the hub sent for it, `complete` last.
async fn run(socket: &mut Socket, id: &str, query: &str) -> Vec<Value> {
    let message = json!({ "id": id, "type": "subscribe", "payload": { "query": query } });
    socket
        .send(Message::text(message.to_string()))
        .await
        .unwrap();
    let mut frames = Vec::new();
    loop {
        let frame = receive(socket).await.unwrap();
        assert_eq!(frame["id"], id, "a frame for another operation: {frame}");
        let complete = frame["type"] == "complete";
        frames.push(frame);
        if complete {
            return frames;
        }
    }
}

/// The one error of a refused operation: its frames are one `next` holding only the error,
/// then `complete`.
fn refusal(frames: &[Value]) -> (String, String) {
    let [next, complete] = frames else {
        panic!("expected next and complete: {frames:?}");
    };
    assert_eq!(next["type"], "next", "{frames:?}");
    assert_eq!(complete["type"], "complete", "{frames:?}");
    assert_eq!(next["payload"]["data"], Value::Null, "{frames:?}");
    error(&next["payload"])
}

/// A socket signed in with `secret` in its `connection_init`.
async fn signed_in_socket(addr: std::net::SocketAddr, secret: &str) -> Socket {
    let mut socket = open(addr, &[]).await;
    let payload = json!({ "authorization": format!("Bearer {secret}") });
    assert_eq!(init(&mut socket, payload).await, Ok(ack()));
    socket
}

const CREATE_KEY: &str = "mutation { createApiKey(input: { name: \"x\", scope: WRITE, madeFor: USER, \
                          expiresAt: \"2030-06-01T00:00:00Z\" }) { secret } }";

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_socket_signed_in_with_a_key_is_refused_the_listed_roots(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let write = secret(&hub, &token, "writer", "WRITE").await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let served = serve_watched(&pool, clock).await;
    let mut socket = signed_in_socket(served.addr, &write).await;
    let documents = [
        ("1", "{ apiKeys { id } }"),
        ("2", CREATE_KEY),
        ("3", "query { ... on Query { apiKeys { id } } }"),
    ];
    for (id, document) in documents {
        let frames = run(&mut socket, id, document).await;
        assert_eq!(refusal(&frames), kind_refused(), "{document}");
    }
    let me = run(&mut socket, "4", "{ me { username } }").await;
    assert_eq!(
        me[0]["payload"]["data"]["me"]["username"], USERNAME,
        "{me:?}"
    );
    assert_eq!(
        count(&pool, "access.api_tokens where kind = 'api_key'").await,
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_socket_signed_in_with_a_read_key_runs_no_mutation(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let read = secret(&hub, &token, "reader", "READ").await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
    let served = serve_watched(&pool, clock).await;
    let mut socket = signed_in_socket(served.addr, &read).await;
    let sites = count(&pool, "devices.sites").await;
    let site = "mutation { createSite(name: \"x\", subnet: \"10.9.0.0/24\") { id } }";
    let frames = run(&mut socket, "1", site).await;
    let read_only = (
        "This token may only read.".to_owned(),
        "FORBIDDEN".to_owned(),
    );
    assert_eq!(refusal(&frames), read_only);
    let me = run(&mut socket, "2", "{ me { username } }").await;
    assert_eq!(
        me[0]["payload"]["data"]["me"]["username"], USERNAME,
        "{me:?}"
    );
    assert_eq!(
        count(&pool, "devices.sites").await,
        sites,
        "no site was made"
    );
}
