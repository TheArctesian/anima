//! A scripted hub: answers each route from a queue and records what it was sent.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use agent_core::wire::RefusalCode;
use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use axum::routing::post;
use reqwest::Url;
use tokio::net::TcpListener;

/// One canned answer.
#[derive(Clone)]
pub enum Reply {
    Status(u16, String),
    Redirect(String),
}

impl Reply {
    pub fn no_content() -> Self {
        Self::Status(204, String::new())
    }

    pub fn refuse(code: RefusalCode) -> Self {
        Self::Status(code.status(), format!(r#"{{"error":"{}"}}"#, code.as_str()))
    }

    pub fn json(status: u16, body: &str) -> Self {
        Self::Status(status, body.to_owned())
    }
}

/// A request the hub received.
#[derive(Clone)]
pub struct Seen {
    pub route: &'static str,
    pub bearer: Option<String>,
    pub cookie: bool,
    pub body: serde_json::Value,
}

#[derive(Default)]
struct Script {
    push: VecDeque<Reply>,
    renew: VecDeque<Reply>,
    enrol: VecDeque<Reply>,
    seen: Vec<Seen>,
}

pub struct FakeHub {
    pub url: Url,
    script: Arc<Mutex<Script>>,
}

type Shared = Arc<Mutex<Script>>;

impl FakeHub {
    pub async fn start() -> Self {
        let script = Shared::default();
        let app = Router::new()
            .route("/agent/v1/push", post(|s, h, b| answer("push", s, h, b)))
            .route("/agent/v1/renew", post(|s, h, b| answer("renew", s, h, b)))
            .route("/agent/v1/enrol", post(|s, h, b| answer("enrol", s, h, b)))
            .with_state(script.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let url =
            Url::parse(&format!("http://{}", listener.local_addr().expect("addr"))).expect("a URL");
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        Self { url, script }
    }

    /// Queues the answers to the next pushes; once empty, a push gets 204.
    pub fn queue_push(&self, replies: impl IntoIterator<Item = Reply>) {
        self.script.lock().expect("lock").push.extend(replies);
    }

    pub fn queue_renew(&self, reply: Reply) {
        self.script.lock().expect("lock").renew.push_back(reply);
    }

    pub fn queue_enrol(&self, reply: Reply) {
        self.script.lock().expect("lock").enrol.push_back(reply);
    }

    /// The requests of one route, in order.
    pub fn seen(&self, route: &str) -> Vec<Seen> {
        let script = self.script.lock().expect("lock");
        script
            .seen
            .iter()
            .filter(|seen| seen.route == route)
            .cloned()
            .collect()
    }
}

#[expect(clippy::unused_async, reason = "axum handlers are async functions")]
async fn answer(
    route: &'static str,
    State(script): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let mut script = script.lock().expect("lock");
    script.seen.push(Seen {
        route,
        bearer: headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::to_owned),
        cookie: headers.contains_key(header::COOKIE),
        body: serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
    });
    let queue = match route {
        "push" => &mut script.push,
        "renew" => &mut script.renew,
        _ => &mut script.enrol,
    };
    let reply = queue.pop_front().unwrap_or_else(|| {
        if route == "push" {
            Reply::no_content()
        } else {
            Reply::refuse(RefusalCode::Unavailable)
        }
    });
    match reply {
        Reply::Status(status, text) => {
            let status = StatusCode::from_u16(status).expect("a status");
            (
                status,
                [
                    (header::CONTENT_TYPE, "application/json"),
                    // A hub that sets a cookie must never see it come back.
                    (header::SET_COOKIE, "session=abc"),
                ],
                text,
            )
                .into_response()
        }
        Reply::Redirect(to) => (StatusCode::FOUND, [(header::LOCATION, to)]).into_response(),
    }
}
