//! A loopback HTTP server standing in for GitHub: it mints installation tokens, serves numbered
//! pages, and records every request it gets. Never the real GitHub (LOG-007, PRJ-018).

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri, header},
    response::Response,
};
use reqwest::Url;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// A canned reply for one path.
#[derive(Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: String,
}

/// What the stub does and what it saw.
pub struct Script {
    /// `(method, path with query, sent a bearer token)` of each request, in order.
    pub seen: Vec<(String, String, bool)>,
    /// The App token (JWT) each mint request was signed in with, so a test can search for it.
    pub app_tokens: Vec<String>,
    pub mint_status: u16,
    pub token: String,
    /// Page n (from 1) holds `pages[n - 1]`, a list of `{ "id", "value" }`.
    pub pages: Vec<Value>,
    /// Replies that win over the rules above, by path.
    pub replies: HashMap<String, Reply>,
    /// SQL run once when its path is first requested, as another writer would.
    pub hooks: HashMap<String, String>,
    /// Paths answered with 200 and a body that never ends, sent in chunks with no length.
    pub endless: HashSet<String>,
}

/// The running stub. Clones share it.
#[derive(Clone)]
pub struct Stub {
    pub script: Arc<Mutex<Script>>,
    pool: PgPool,
    pub base: Url,
}

impl Stub {
    /// Starts the stub on a free loopback port; `pool` runs its hooks.
    pub async fn start(pool: &PgPool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        let script = Script {
            seen: Vec::new(),
            app_tokens: Vec::new(),
            mint_status: 201,
            // A token shape of the test's own: never assume a length or a prefix.
            token: format!("stubtoken-{}", Uuid::now_v7().simple()),
            pages: Vec::new(),
            replies: HashMap::new(),
            hooks: HashMap::new(),
            endless: HashSet::new(),
        };
        let stub = Self {
            script: Arc::new(Mutex::new(script)),
            pool: pool.clone(),
            base: Url::parse(&format!("http://{address}")).unwrap(),
        };
        let app = Router::new().fallback(answer).with_state(stub.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        stub
    }

    pub fn with<T>(&self, f: impl FnOnce(&mut Script) -> T) -> T {
        f(&mut self.script.lock().unwrap())
    }

    /// The paths of every request so far.
    pub fn paths(&self) -> Vec<String> {
        self.with(|s| s.seen.iter().map(|(_, path, _)| path.clone()).collect())
    }

    /// The paths of the GET requests so far: the provider's requests, not the mints.
    pub fn gets(&self) -> Vec<String> {
        self.with(|s| {
            s.seen
                .iter()
                .filter(|(method, _, _)| method == "GET")
                .map(|(_, path, _)| path.clone())
                .collect()
        })
    }

    pub fn token(&self) -> String {
        self.with(|s| s.token.clone())
    }

    /// Sets the pages, each a list of `(id, value)`.
    pub fn set_pages(&self, pages: &[&[(&str, &str)]]) {
        let pages = pages
            .iter()
            .map(|items| {
                Value::Array(
                    items
                        .iter()
                        .map(|(id, value)| json!({ "id": id, "value": value }))
                        .collect(),
                )
            })
            .collect();
        self.with(|s| s.pages = pages);
    }

    pub fn reply(&self, path: &str, reply: Reply) {
        self.with(|s| s.replies.insert(path.to_owned(), reply));
    }

    /// Answers `path` with a body that never ends: only a reader that stops at its cap returns.
    pub fn endless(&self, path: &str) {
        self.with(|s| s.endless.insert(path.to_owned()));
    }

    pub fn hook(&self, path: &str, sql: &str) {
        self.with(|s| s.hooks.insert(path.to_owned(), sql.to_owned()));
    }
}

fn respond(status: u16, headers: &[(&'static str, String)], body: String) -> Response {
    let mut response = Response::builder().status(StatusCode::from_u16(status).unwrap());
    for (name, value) in headers {
        response = response.header(*name, value);
    }
    response
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// 200 with 64 KiB chunks of spaces, forever: generated as it is sent, never held.
fn endless() -> Response {
    let chunk = Bytes::from(vec![b' '; 64 * 1024]);
    let chunks = futures_util::stream::repeat_with(move || Ok::<_, std::io::Error>(chunk.clone()));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from_stream(chunks))
        .unwrap()
}

async fn answer(
    State(stub): State<Stub>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let path = uri.path_and_query().map_or("/", |p| p.as_str()).to_owned();
    let bearer = headers
        .get(header::AUTHORIZATION)
        .is_some_and(|v| v.as_bytes().starts_with(b"Bearer "));
    let app_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|_| method == Method::POST)
        .map(str::to_owned);
    let hook = stub.with(|s| {
        s.seen.push((method.to_string(), path.clone(), bearer));
        s.app_tokens.extend(app_token);
        s.hooks.remove(uri.path())
    });
    if let Some(sql) = hook {
        sqlx::query(&sql).execute(&stub.pool).await.unwrap();
    }
    if stub.with(|s| s.endless.contains(uri.path())) {
        return endless();
    }
    let (reply, mint_status, token, pages) = stub.with(|s| {
        (
            s.replies.get(uri.path()).cloned(),
            s.mint_status,
            s.token.clone(),
            s.pages.clone(),
        )
    });
    if let Some(reply) = reply {
        return respond(reply.status, &reply.headers, reply.body);
    }
    if method == Method::POST && uri.path().starts_with("/app/installations/") {
        let body = json!({ "token": token, "expires_at": "2099-01-01T00:00:00Z" });
        return respond(mint_status, &[], body.to_string());
    }
    let page: Option<usize> = uri
        .path()
        .strip_prefix("/pages/")
        .and_then(|n| n.parse().ok());
    match page.and_then(|n| pages.get(n - 1).map(|items| (n, items))) {
        Some((n, items)) => {
            let next = (n < pages.len()).then_some(n + 1);
            respond(
                200,
                &[],
                json!({ "items": items, "next": next }).to_string(),
            )
        }
        None => respond(404, &[], "{}".to_owned()),
    }
}
