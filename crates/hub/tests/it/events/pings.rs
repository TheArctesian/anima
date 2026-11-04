//! A client that sends RFC 6455 Pings and never reads cannot make the hub buffer without bound:
//! the pongs the hub owes it stop growing at a fixed size (H-6(b), API-003).

use std::{net::SocketAddr, sync::Arc, time::Duration};

use anima_hub::clock::{Clock, FixedClock};
use async_graphql::futures_util::{SinkExt as _, StreamExt as _};
use serde_json::json;
use sqlx::PgPool;
use tokio::net::{TcpSocket, TcpStream};
use tokio_tungstenite::{
    WebSocketStream, client_async,
    tungstenite::{Message, client::IntoClientRequest as _, http::HeaderValue},
};

use super::client::{DEADLINE, bearer, serve_watched};
use crate::hub::{Hub, set_operator, start};

/// The largest control frame payload RFC 6455 allows.
const PING: [u8; 125] = [7; 125];
/// What the client sends: far more pongs than the hub may hold for it.
const FLOOD: usize = 24 * 1024 * 1024;
/// The most pong bytes the client may find waiting once it reads: the hub's 2 MiB write buffer
/// and the kernel's socket buffers, with room to spare. Without the hub's limit it finds all
/// of them, close to [`FLOOD`].
const BOUNDED: usize = 12 * 1024 * 1024;
/// A pong frame from the hub: two header bytes and the payload.
const PONG_FRAME: usize = 2 + PING.len();

type Raw = WebSocketStream<TcpStream>;

/// A `graphql-transport-ws` connection whose receive buffer is as small as the system allows,
/// so that the hub's writes block at once.
async fn open_small(addr: SocketAddr) -> Raw {
    let tcp = TcpSocket::new_v4().unwrap();
    tcp.set_recv_buffer_size(1024).unwrap();
    let stream = tcp.connect(addr).await.unwrap();
    let mut request = format!("ws://{addr}/graphql")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_static("graphql-transport-ws"),
    );
    let handshake = tokio::time::timeout(DEADLINE, client_async(request, stream)).await;
    handshake.expect("the handshake ends in time").unwrap().0
}

/// Sends [`FLOOD`] bytes of Pings without reading, then reads; how many pongs were waiting.
async fn pongs_after_flood(socket: &mut Raw) -> usize {
    let flood = async {
        for sent in 0..FLOOD / (PING.len() + 6) {
            socket
                .feed(Message::Ping(PING.to_vec().into()))
                .await
                .unwrap();
            if sent % 256 == 0 {
                socket.flush().await.unwrap();
            }
        }
        socket.flush().await.unwrap();
    };
    tokio::time::timeout(DEADLINE * 6, flood)
        .await
        .expect("the hub keeps reading Pings");
    let mut pongs = 0;
    while let Ok(Some(Ok(message))) =
        tokio::time::timeout(Duration::from_secs(2), socket.next()).await
    {
        assert!(matches!(message, Message::Pong(_)), "{message:?}");
        pongs += 1;
        assert!(pongs * PONG_FRAME <= FLOOD, "more pongs than Pings");
    }
    pongs
}

fn served_clock() -> Arc<dyn Clock> {
    Arc::new(FixedClock(start()))
}

/// Long enough that neither the sign-in deadline nor a re-check ends the connection mid-test.
const PATIENT: Duration = Duration::from_secs(120);

#[sqlx::test(migrations = "./migrations")]
async fn h_6_pings_before_sign_in_buffer_a_bounded_amount(pool: PgPool) {
    let served = serve_watched(&pool, served_clock(), PATIENT, PATIENT).await;
    let mut socket = open_small(served.addr).await;
    let pongs = pongs_after_flood(&mut socket).await;
    assert!(pongs > 0, "the hub answers Pings");
    assert!(pongs * PONG_FRAME <= BOUNDED, "{pongs} pongs were buffered");
}

#[sqlx::test(migrations = "./migrations")]
async fn h_6_pings_after_sign_in_buffer_a_bounded_amount(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let served = serve_watched(&pool, served_clock(), PATIENT, PATIENT).await;
    let mut socket = open_small(served.addr).await;
    let init = json!({ "type": "connection_init", "payload": bearer(&token) });
    socket.send(Message::text(init.to_string())).await.unwrap();
    let ack = tokio::time::timeout(DEADLINE, socket.next()).await.unwrap();
    let ack = ack.unwrap().unwrap().into_text().unwrap();
    assert_eq!(ack.as_str(), r#"{"type":"connection_ack"}"#);
    let pongs = pongs_after_flood(&mut socket).await;
    assert!(pongs > 0, "the hub answers Pings");
    assert!(pongs * PONG_FRAME <= BOUNDED, "{pongs} pongs were buffered");
}
