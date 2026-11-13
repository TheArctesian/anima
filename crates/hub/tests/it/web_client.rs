//! The web client mounted as the hub's fallback, as `main.rs` wires it (D-2, K7): explicit
//! routes win, the fallback is logged without its path and sits behind the NET-002 layer.

use std::{
    io,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};

use anima_hub::{
    clock::{Clock, SystemClock},
    fleet::{
        access::{self, PublicOrigin},
        events::{Bus, process_log},
    },
    router,
    schema::build_schema,
    web_fallback,
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{HeaderMap, Request, StatusCode, header},
};
use http_body_util::BodyExt as _;
use ipnet::IpNet;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tempfile::TempDir;
use tower::ServiceExt as _;

use crate::hub::dependencies;

const INDEX_HTML: &str = "<!doctype html><script>start()</script>";
const MARKER: &str = "SECRET-PATH-MARKER";

/// A web root holding only `index.html`.
fn web_root() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), INDEX_HTML).unwrap();
    dir
}

fn origin() -> PublicOrigin {
    PublicOrigin::new("https://hub.anima.ing".to_owned())
}

/// The hub's routes on `pool` with the web client at `root`, merged as `main.rs` merges them.
fn hub(pool: &PgPool, root: Option<&Path>) -> Router {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let schema = build_schema(dependencies(pool, clock.clone(), Bus::new(1)));
    router(schema, pool.clone(), clock).merge(web_fallback(root, &origin()))
}

/// A pool that never connects: these requests need no database.
fn no_database() -> PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap()
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn get(path: &str) -> Request<Body> {
    Request::get(path).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn d_2_health_still_answers_ok_with_a_web_root() {
    let root = web_root();
    let app = hub(&no_database(), Some(root.path()));
    let (status, headers, body) = send(&app, get("/health")).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "ok"));
    assert!(headers.get(header::CONTENT_SECURITY_POLICY).is_none());
}

#[tokio::test]
async fn d_2_post_graphql_still_reaches_graphql_with_a_web_root() {
    let root = web_root();
    let app = hub(&no_database(), Some(root.path()));
    let request = Request::post("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "query": "{ nosuch }" }).to_string()))
        .unwrap();
    let (status, headers, body) = send(&app, request).await;
    assert_eq!(status, StatusCode::OK);
    let content_type = headers[header::CONTENT_TYPE].to_str().unwrap();
    assert!(content_type.contains("json"), "{content_type}");
    assert!(headers.get(header::CONTENT_SECURITY_POLICY).is_none());
    let answer: Value = serde_json::from_str(&body).unwrap();
    assert!(answer["errors"].is_array(), "{answer}");
}

#[tokio::test]
async fn d_2_explicit_routes_keep_their_method_answers_with_a_web_root() {
    let root = web_root();
    let app = hub(&no_database(), Some(root.path()));
    // `GET /graphql` that is not an upgrade, and `POST /health`: the routes answer 405.
    assert_eq!(
        send(&app, get("/graphql")).await.0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    let post = Request::post("/health").body(Body::empty()).unwrap();
    assert_eq!(send(&app, post).await.0, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn d_2_an_unknown_path_gets_index_html_with_a_web_root() {
    let root = web_root();
    let app = hub(&no_database(), Some(root.path()));
    let (status, headers, body) = send(&app, get("/devices/0190a000")).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, INDEX_HTML));
    assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
}

#[tokio::test]
async fn d_2_without_a_web_root_an_unknown_path_is_404() {
    let app = hub(&no_database(), None);
    for path in ["/", "/devices", "/index.html"] {
        let (status, _, body) = send(&app, get(path)).await;
        assert_eq!(
            (status, body.as_str()),
            (StatusCode::NOT_FOUND, ""),
            "{path}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_the_layer_covers_the_web_client(pool: PgPool) {
    let networks: Vec<IpNet> = vec!["10.0.0.0/8".parse().unwrap()];
    access::permit_networks(&pool, &networks).await.unwrap();
    let root = web_root();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let edge = access::Edge::new(pool.clone(), Bus::new(4), clock);
    let app = hub(&pool, Some(root.path())).layer(access::permitted_networks_layer(edge));
    let from = |peer: &str| {
        let mut request = get("/devices");
        let peer: SocketAddr = peer.parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        request
    };

    let (refused, _, body) = send(&app, from("192.0.2.9:50000")).await;
    assert_eq!(refused, StatusCode::FORBIDDEN);
    assert!(!body.contains(INDEX_HTML));
    let (permitted, _, body) = send(&app, from("10.1.2.3:50000")).await;
    assert_eq!((permitted, body.as_str()), (StatusCode::OK, INDEX_HTML));
}

/// Collects everything the process log writes.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn log_007_a_web_client_request_is_logged_without_its_path() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in wiring::uri_log: a second dispatcher keeps other test threads from caching this
    // test's callsites as disabled.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let root = web_root();
    let app = hub(&no_database(), Some(root.path()));
    for path in [format!("/{MARKER}"), format!("/_app/{MARKER}.js?{MARKER}")] {
        send(&app, get(&path)).await;
    }

    let log = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("[no route]"), "{log}");
    assert_eq!(log.matches("request served").count(), 2, "{log}");
    assert!(!log.contains(MARKER), "{log}");
}
