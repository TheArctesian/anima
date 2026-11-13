//! A scripted hub: `POST /graphql` answers each operation from its script and records what it
//! was sent.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use axum::routing::post;
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// A request the hub received.
#[derive(Clone, Debug)]
pub struct Seen {
    pub operation: String,
    pub authorization: Option<String>,
    /// The `Anima-Actor` header, when the request declared itself (LOG-016).
    pub actor: Option<String>,
    pub variables: Value,
}

#[derive(Default)]
struct Script {
    /// Answers by operation name; the last one repeats.
    answers: HashMap<String, VecDeque<Value>>,
    seen: Vec<Seen>,
}

type Shared = Arc<Mutex<Script>>;

pub struct FakeHub {
    pub origin: String,
    script: Shared,
}

impl FakeHub {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let origin = format!("http://{}", listener.local_addr().expect("an address"));
        let script = Shared::default();
        let app = Router::new()
            .route("/graphql", post(answer))
            .with_state(script.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        Self { origin, script }
    }

    /// Answers `operation` with `body` (after any answers queued before it).
    pub fn answer(&self, operation: &str, body: Value) -> &Self {
        let mut script = self.script.lock().expect("script");
        script
            .answers
            .entry(operation.to_owned())
            .or_default()
            .push_back(body);
        self
    }

    pub fn seen(&self) -> Vec<Seen> {
        self.script.lock().expect("script").seen.clone()
    }
}

/// `{"data": data}`.
pub fn data(data: Value) -> Value {
    Value::Object(serde_json::Map::from_iter([("data".to_owned(), data)]))
}

/// A GraphQL error with a K7 code.
pub fn refusal(code: &str, message: &str) -> Value {
    json!({ "data": null, "errors": [{ "message": message, "extensions": { "code": code } }] })
}

async fn answer(State(script): State<Shared>, headers: HeaderMap, body: String) -> String {
    let request: Value = serde_json::from_str(&body).expect("the request is JSON");
    let operation = request["operationName"]
        .as_str()
        .map_or_else(|| operation_of(&request["query"]), str::to_owned);
    let mut script = script.lock().expect("script");
    script.seen.push(Seen {
        operation: operation.clone(),
        authorization: headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        actor: headers
            .get("anima-actor")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        variables: request["variables"].clone(),
    });
    let reply = match script.answers.get_mut(&operation) {
        Some(queue) if queue.len() > 1 => queue.pop_front(),
        Some(queue) => queue.front().cloned(),
        None => None,
    };
    reply
        .unwrap_or_else(|| refusal("INTERNAL", &format!("no answer for {operation}")))
        .to_string()
}

/// The name of a document sent without `operationName` (`anima api`): the word after
/// `query` or `mutation`.
fn operation_of(query: &Value) -> String {
    let text = query.as_str().unwrap_or_default();
    text.split_whitespace()
        .nth(1)
        .and_then(|word| word.split(['(', '{']).next())
        .unwrap_or_default()
        .to_owned()
}
