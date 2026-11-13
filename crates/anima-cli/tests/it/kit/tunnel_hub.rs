//! A scripted hub with tunnels: `POST /graphql` as `FakeHub` answers it, plus `/terminal` and
//! `/files` WebSocket routes (K4 Sections 8 and 9) that redeem each ticket once and play a script.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{RawQuery, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, post};
use serde_json::{Value, json};
use tokio::net::TcpListener;

use super::fake_hub::refusal;

/// The terminal ticket the scripts hand out; base64url, 43 characters, as K4 Section 1.
pub const TERMINAL_TICKET: &str = "TerminalTicket0123456789abcdefghijklmnopqrs";
/// The files ticket.
pub const FILES_TICKET: &str = "FilesTicket_0123456789abcdefghijklmnopqrstu";

/// What the terminal does after the first message.
#[derive(Clone)]
pub enum Terminal {
    /// Waits for one input frame, sends `output`, then `end` with `reason` and closes `code`.
    Speak {
        output: Vec<u8>,
        reason: &'static str,
        code: u16,
    },
    /// Stays open until the client closes (the files tunnel's session).
    Hold,
}

/// What the files tunnel answers.
#[derive(Clone, Default)]
pub struct Files {
    /// The `entries` message for any `list`.
    pub listing: Value,
    /// The file any `download` sends.
    pub file: Vec<u8>,
    /// A `refused` reason for every request instead.
    pub refuse: Option<&'static str>,
}

#[derive(Default)]
struct Script {
    answers: HashMap<String, VecDeque<Value>>,
    operations: Vec<String>,
    unspent: HashSet<String>,
    terminal: Option<Terminal>,
    files: Files,
    /// Each tunnel request's query, and every message the client sent on a tunnel.
    queries: Vec<String>,
    received: Vec<String>,
}

type Shared = Arc<Mutex<Script>>;

pub struct TunnelHub {
    pub origin: String,
    script: Shared,
}

impl TunnelHub {
    pub async fn start(terminal: Terminal, files: Files) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let origin = format!("http://{}", listener.local_addr().expect("an address"));
        let script = Shared::new(Mutex::new(Script {
            unspent: HashSet::from([TERMINAL_TICKET.to_owned(), FILES_TICKET.to_owned()]),
            terminal: Some(terminal),
            files,
            ..Script::default()
        }));
        let app = Router::new()
            .route("/graphql", post(graphql))
            .route("/terminal", any(terminal_route))
            .route("/files", any(files_route))
            .with_state(script.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        Self { origin, script }
    }

    /// Answers `operation` with `body`; the last answer repeats.
    pub fn answer(&self, operation: &str, body: Value) -> &Self {
        let mut script = self.script.lock().expect("script");
        let queue = script.answers.entry(operation.to_owned()).or_default();
        queue.push_back(body);
        self
    }

    /// The GraphQL operations, in order.
    pub fn operations(&self) -> Vec<String> {
        self.script.lock().expect("script").operations.clone()
    }

    /// The query string of each tunnel request.
    pub fn queries(&self) -> Vec<String> {
        self.script.lock().expect("script").queries.clone()
    }

    /// Every message the client sent on a tunnel: text as it is, binary as `binary:<utf8>`.
    pub fn received(&self) -> Vec<String> {
        self.script.lock().expect("script").received.clone()
    }
}

async fn graphql(State(script): State<Shared>, body: String) -> String {
    let request: Value = serde_json::from_str(&body).expect("the request is JSON");
    let operation = request["operationName"].as_str().unwrap_or("").to_owned();
    let mut script = script.lock().expect("script");
    script.operations.push(operation.clone());
    let reply = match script.answers.get_mut(&operation) {
        Some(queue) if queue.len() > 1 => queue.pop_front(),
        Some(queue) => queue.front().cloned(),
        None => None,
    };
    reply
        .unwrap_or_else(|| refusal("INTERNAL", &format!("no answer for {operation}")))
        .to_string()
}

/// Spends the ticket in `query` (K4 Section 1: once, valid or not); whether it was `ticket`.
fn redeem(script: &Shared, query: Option<String>, ticket: &str) -> bool {
    let query = query.unwrap_or_default();
    let mut script = script.lock().expect("script");
    script.queries.push(query.clone());
    let presented = query.strip_prefix("token=").unwrap_or_default();
    presented == ticket && script.unspent.remove(ticket)
}

async fn terminal_route(
    State(script): State<Shared>,
    RawQuery(query): RawQuery,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !redeem(&script, query, TERMINAL_TICKET) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade
        .protocols(["anima-terminal"])
        .on_upgrade(move |socket| play_terminal(socket, script))
}

async fn files_route(
    State(script): State<Shared>,
    RawQuery(query): RawQuery,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !redeem(&script, query, FILES_TICKET) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade
        .protocols(["anima-files"])
        .on_upgrade(move |socket| play_files(socket, script))
}

/// The next client message, recorded; `None` when the client closed or went quiet.
async fn heard(socket: &mut WebSocket, script: &Shared) -> Option<Message> {
    let message = tokio::time::timeout(Duration::from_secs(10), socket.recv())
        .await
        .ok()??
        .ok()?;
    let note = match &message {
        Message::Text(text) => text.to_string(),
        Message::Binary(bytes) => format!("binary:{}", String::from_utf8_lossy(bytes)),
        Message::Close(_) => return None,
        Message::Ping(_) | Message::Pong(_) => return Some(message),
    };
    script.lock().expect("script").received.push(note);
    Some(message)
}

async fn play_terminal(mut socket: WebSocket, script: Shared) {
    let terminal = script.lock().expect("script").terminal.clone();
    let _first = heard(&mut socket, &script).await;
    match terminal {
        Some(Terminal::Speak {
            output,
            reason,
            code,
        }) => {
            while let Some(message) = heard(&mut socket, &script).await {
                if matches!(message, Message::Binary(_)) {
                    break;
                }
            }
            let end = json!({ "type": "end", "reason": reason }).to_string();
            let _sent = socket.send(Message::Binary(output.into())).await;
            let _sent = socket.send(Message::Text(end.into())).await;
            let close = CloseFrame {
                code,
                reason: "".into(),
            };
            let _sent = socket.send(Message::Close(Some(close))).await;
        }
        Some(Terminal::Hold) | None => while heard(&mut socket, &script).await.is_some() {},
    }
}

async fn play_files(mut socket: WebSocket, script: Shared) {
    let files = script.lock().expect("script").files.clone();
    let done = || Message::Text(r#"{"type":"done"}"#.into());
    // Upload bytes still to come; `done` follows the last.
    let mut uploading: u64 = 0;
    while let Some(message) = heard(&mut socket, &script).await {
        let text = match message {
            Message::Text(text) => text,
            Message::Binary(bytes) if uploading > 0 => {
                uploading = uploading.saturating_sub(bytes.len() as u64);
                if uploading == 0 {
                    let _sent = socket.send(done()).await;
                }
                continue;
            }
            _ => continue,
        };
        let request: Value = serde_json::from_str(&text).unwrap_or_default();
        let replies = match (files.refuse, request["type"].as_str()) {
            (_, Some("ping")) => Vec::new(),
            (None, Some("upload")) => {
                uploading = request["size"].as_u64().unwrap_or(0);
                if uploading == 0 {
                    vec![done()]
                } else {
                    Vec::new()
                }
            }
            (Some(reason), _) => vec![Message::Text(
                json!({ "type": "refused", "reason": reason })
                    .to_string()
                    .into(),
            )],
            (None, Some("list")) => vec![Message::Text(files.listing.to_string().into())],
            (None, Some("download")) => {
                let size = json!({ "type": "download", "size": files.file.len() });
                let mut replies = vec![Message::Text(size.to_string().into())];
                for chunk in files.file.chunks(4) {
                    replies.push(Message::Binary(chunk.to_vec().into()));
                }
                replies.push(done());
                replies
            }
            _ => Vec::new(),
        };
        for reply in replies {
            let _sent = socket.send(reply).await;
        }
    }
}
