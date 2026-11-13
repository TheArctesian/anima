//! A hub served on a loopback port and a `graphql-transport-ws` client for it. Every read and
//! connect has a deadline, so a hanging socket fails the test instead of stalling the suite.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anima_hub::{
    clock::Clock,
    fleet::{
        access,
        events::{Bus, EventSummary, Severity, Signal, Source},
    },
    ids::EventId,
    router,
    schema::build_schema,
};
use async_graphql::futures_util::{SinkExt as _, StreamExt as _};
use axum::{Router, extract::WebSocketUpgrade, routing::get};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{net::TcpStream, task::JoinHandle};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{self, Message, client::IntoClientRequest as _, http::HeaderValue},
};
use uuid::Uuid;

use crate::hub::dependencies;

/// The longest any socket step may take.
pub const DEADLINE: Duration = Duration::from_secs(10);

/// A clock the test moves.
#[derive(Clone)]
pub struct TestClock(Arc<Mutex<DateTime<Utc>>>);

impl TestClock {
    pub fn at(now: DateTime<Utc>) -> Self {
        Self(Arc::new(Mutex::new(now)))
    }

    pub fn set(&self, now: DateTime<Utc>) {
        *self.0.lock().unwrap() = now;
    }
}

impl Clock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

/// A hub listening on a loopback port; dropping it stops the server.
pub struct Served {
    pub addr: SocketAddr,
    pub bus: Bus,
    task: JoinHandle<()>,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn spawn(app: Router, bus: Bus) -> Served {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let service = app.into_make_service_with_connect_info::<SocketAddr>();
    let task = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    Served { addr, bus, task }
}

/// The hub's own router, as `main.rs` serves it.
pub async fn serve_router(pool: &PgPool, clock: Arc<dyn Clock>) -> Served {
    let bus = Bus::new(16);
    let schema = build_schema(dependencies(pool, clock.clone(), bus.clone()));
    spawn(router(schema, pool.clone(), clock), bus).await
}

/// Only the WebSocket route, with a short re-check interval and `connection_init` timeout.
pub async fn serve_watched(
    pool: &PgPool,
    clock: Arc<dyn Clock>,
    recheck: Duration,
    init_timeout: Duration,
) -> Served {
    let bus = Bus::new(16);
    let schema = build_schema(dependencies(pool, clock.clone(), bus.clone()));
    let checks =
        access::SocketAccess::new(pool.clone(), clock, None).with_timings(recheck, init_timeout);
    let route = get(move |upgrade: WebSocketUpgrade| {
        let (schema, checks) = (schema.clone(), checks.clone());
        async move { access::accept_socket(upgrade, schema, checks) }
    });
    spawn(Router::new().route("/graphql", route), bus).await
}

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Opens a WebSocket to `path` that offers `subprotocol`.
pub async fn connect(
    addr: SocketAddr,
    path: &str,
    subprotocol: &'static str,
) -> Result<Socket, tungstenite::Error> {
    let mut request = format!("ws://{addr}{path}").into_client_request().unwrap();
    request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_static(subprotocol),
    );
    let connected = tokio::time::timeout(DEADLINE, connect_async(request))
        .await
        .expect("the handshake ends before the deadline");
    connected.map(|(socket, _)| socket)
}

/// A `graphql-transport-ws` connection to `/graphql`.
pub async fn open(addr: SocketAddr) -> Socket {
    connect(addr, "/graphql", "graphql-transport-ws")
        .await
        .unwrap()
}

/// What the hub sent next.
#[derive(Debug, PartialEq)]
pub enum Received {
    Json(Value),
    /// The hub closed the connection with this code, or without a close frame (`None`).
    Closed(Option<u16>),
}

pub async fn send(socket: &mut Socket, message: &Value) {
    let text = message.to_string();
    tokio::time::timeout(DEADLINE, socket.send(Message::text(text)))
        .await
        .expect("the send ends before the deadline")
        .unwrap();
}

pub async fn receive(socket: &mut Socket) -> Received {
    loop {
        let next = tokio::time::timeout(DEADLINE, socket.next())
            .await
            .expect("a message before the deadline");
        match next {
            Some(Ok(Message::Text(text))) => {
                return Received::Json(serde_json::from_str(&text).unwrap());
            }
            Some(Ok(Message::Close(frame))) => {
                return Received::Closed(frame.map(|frame| u16::from(frame.code)));
            }
            Some(Ok(_)) => {}
            Some(Err(_)) | None => return Received::Closed(None),
        }
    }
}

/// Sends `connection_init` with `payload` (none when it is null) and returns the answer.
pub async fn init(socket: &mut Socket, payload: Value) -> Received {
    let message = if payload.is_null() {
        json!({ "type": "connection_init" })
    } else {
        json!({ "type": "connection_init", "payload": payload })
    };
    send(socket, &message).await;
    receive(socket).await
}

pub fn bearer(token: &str) -> Value {
    json!({ "authorization": format!("Bearer {token}") })
}

pub fn ack() -> Received {
    Received::Json(json!({ "type": "connection_ack" }))
}

/// Publishes an event straight onto the bus, as `events::record` does after its insert.
pub fn publish(bus: &Bus, severity: Severity, message: &str) {
    bus.publish(Signal::Event(EventSummary {
        id: EventId::from(Uuid::now_v7()),
        at: crate::hub::start(),
        severity,
        source: Source::Hub,
        device: None,
        message: message.to_owned(),
        actor: Some(anima_hub::fleet::events::ActorKind::Hub),
    }));
}

const READY: &str = "ready";

/// Signs in with `token`, subscribes to `eventAdded` at `min_severity` as id `1`, and returns
/// once events flow: the subscription takes its bus receiver only when it first runs, so marker
/// events are published until one arrives.
pub async fn subscribe_events(socket: &mut Socket, bus: &Bus, token: &str, min_severity: &str) {
    assert_eq!(init(socket, bearer(token)).await, ack());
    let query = format!(
        "subscription {{ eventAdded(minSeverity: {min_severity}) {{ severity message }} }}"
    );
    let subscribe = json!({ "id": "1", "type": "subscribe", "payload": { "query": query } });
    send(socket, &subscribe).await;
    let ready = async {
        loop {
            publish(bus, Severity::Error, READY);
            let wait = tokio::time::timeout(Duration::from_millis(100), socket.next()).await;
            match wait {
                // Nothing yet: the subscription may not hold its receiver; publish again.
                Err(_) | Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
                Ok(Some(Ok(Message::Text(text)))) => {
                    let message: Value = serde_json::from_str(&text).unwrap();
                    assert_eq!(message["type"], "next", "{message}");
                    return;
                }
                closed => panic!("the connection ended before events flowed: {closed:?}"),
            }
        }
    };
    tokio::time::timeout(DEADLINE, ready)
        .await
        .expect("the subscription delivers before the deadline");
}

/// The next `eventAdded` payload, skipping the markers of [`subscribe_events`]; or how the
/// connection closed.
pub async fn next_event(socket: &mut Socket) -> Received {
    loop {
        match receive(socket).await {
            Received::Json(message)
                if message["payload"]["data"]["eventAdded"]["message"] == READY => {}
            Received::Json(message) => {
                assert_eq!(
                    (&message["type"], &message["id"]),
                    (&json!("next"), &json!("1"))
                );
                return Received::Json(message["payload"]["data"]["eventAdded"].clone());
            }
            closed @ Received::Closed(_) => return closed,
        }
    }
}
