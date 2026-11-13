//! A scripted hub for subscriptions: `GET /graphql` upgrades to `graphql-transport-ws`, takes
//! `connection_init`, sends a ping, takes the `subscribe`, and then plays its script.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::http::Uri;
use axum::response::Response;
use axum::routing::get;
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// What the hub does once the client has subscribed.
#[derive(Clone, Debug)]
pub enum Script {
    /// Sends each value as the `data` of a `next` frame, then `complete`.
    Next(Vec<Value>),
    /// Sends an `error` frame with these GraphQL errors.
    Error(Value),
    /// Closes the socket with 4403 at `connection_init`, as for a token it does not know.
    Refuse,
}

/// What the client sent.
#[derive(Clone, Debug, Default)]
pub struct Received {
    /// The request path and query, where a token must never be.
    pub uri: String,
    /// The payload of `connection_init`.
    pub init: Value,
    /// The payload of `subscribe`: `query`, `variables` and `operationName`.
    pub subscribe: Value,
    /// How many `pong` frames answered the hub's ping.
    pub pongs: usize,
}

struct Shared {
    script: Script,
    received: Mutex<Received>,
}

pub struct SubscriptionHub {
    pub origin: String,
    shared: Arc<Shared>,
}

impl SubscriptionHub {
    pub async fn start(script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let origin = format!("http://{}", listener.local_addr().expect("an address"));
        let shared = Arc::new(Shared {
            script,
            received: Mutex::default(),
        });
        let app = Router::new()
            .route("/graphql", get(upgrade))
            .with_state(shared.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        Self { origin, shared }
    }

    /// What the client has sent so far.
    pub fn received(&self) -> Received {
        self.shared.received.lock().expect("received").clone()
    }
}

async fn upgrade(
    State(shared): State<Arc<Shared>>,
    uri: Uri,
    upgrade: WebSocketUpgrade,
) -> Response {
    shared.received.lock().expect("received").uri = uri.to_string();
    upgrade
        .protocols(["graphql-transport-ws"])
        .on_upgrade(move |socket| play(socket, shared))
}

async fn play(mut socket: WebSocket, shared: Arc<Shared>) {
    let Some(init) = read(&mut socket).await else {
        return;
    };
    shared.received.lock().expect("received").init = init["payload"].clone();
    if matches!(shared.script, Script::Refuse) {
        let frame = CloseFrame {
            code: 4403,
            reason: "Forbidden".into(),
        };
        socket.send(Message::Close(Some(frame))).await.ok();
        return;
    }
    send(&mut socket, json!({ "type": "connection_ack" })).await;
    send(&mut socket, json!({ "type": "ping" })).await;
    if await_subscription(&mut socket, &shared).await {
        answer(&mut socket, &shared.script).await;
        // Let the client read what was sent before the socket drops.
        while read(&mut socket).await.is_some() {}
    }
}

/// Reads until the client has subscribed and answered the ping; false when it left first.
async fn await_subscription(socket: &mut WebSocket, shared: &Shared) -> bool {
    let mut subscribed = false;
    while !subscribed || shared.received.lock().expect("received").pongs == 0 {
        let Some(frame) = read(socket).await else {
            return false;
        };
        match frame["type"].as_str() {
            Some("pong") => shared.received.lock().expect("received").pongs += 1,
            Some("subscribe") => {
                shared.received.lock().expect("received").subscribe = frame["payload"].clone();
                subscribed = true;
            }
            _ => {}
        }
    }
    true
}

async fn answer(socket: &mut WebSocket, script: &Script) {
    match script {
        Script::Next(answers) => {
            for data in answers {
                let payload = json!({ "data": data });
                send(
                    socket,
                    json!({ "id": "1", "type": "next", "payload": payload }),
                )
                .await;
            }
            send(socket, json!({ "id": "1", "type": "complete" })).await;
        }
        Script::Error(errors) => {
            send(
                socket,
                json!({ "id": "1", "type": "error", "payload": errors }),
            )
            .await;
        }
        Script::Refuse => {}
    }
}

async fn send(socket: &mut WebSocket, frame: Value) {
    socket
        .send(Message::Text(frame.to_string().into()))
        .await
        .ok();
}

/// The next text frame as JSON; `None` when the client has gone.
async fn read(socket: &mut WebSocket) -> Option<Value> {
    loop {
        match socket.recv().await? {
            Ok(Message::Text(text)) => return serde_json::from_str(&text).ok(),
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => {}
        }
    }
}
