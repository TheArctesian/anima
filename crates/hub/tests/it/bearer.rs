//! `GET /metrics` behind the bearer layer as `main.rs` serves it, and the agent secrets the
//! access guard refuses (WP-43 A-9, R-092, INT-001, INT-003, D-24, NET-002).

mod agent;
mod lookup;
mod metrics;

use std::{collections::BTreeMap, net::SocketAddr, path::Path, sync::Arc};

use anima_hub::{
    access, all_routes,
    clock::{Clock, FixedClock},
    config,
    events::Bus,
    schema::Dependencies,
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, DurationRound as _, TimeDelta, Utc};
use http_body_util::BodyExt as _;
use ipnet::IpNet;
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use tempfile::TempDir;
use tower::ServiceExt as _;

use crate::hub::{dependencies, set_operator};

/// The public origin the test hub serves.
const PUBLIC: &str = "https://hub.anima.ing";
/// A client inside the permitted networks.
const PERMITTED: &str = "10.10.0.5:40000";

/// The body of the web client's `index.html`.
const INDEX_HTML: &str = "<!doctype html><title>Anima</title>";

/// The hub's routes at `now` with a web root holding only `index.html`, kept while the
/// returned folder lives: see [`app_at`].
async fn app(pool: &PgPool, now: DateTime<Utc>) -> (Router, TempDir) {
    let web = tempfile::tempdir().unwrap();
    std::fs::write(web.path().join("index.html"), INDEX_HTML).unwrap();
    (app_at(pool, now, Some(web.path())).await, web)
}

/// The hub's routes at `now` as `main.rs` serves them, [`all_routes`]: GraphQL at the public
/// origin, the tunnels, `/push`, `/metrics` and the web client at `root` when there is one,
/// all behind the permitted networks `10.10.0.0/16`.
async fn app_at(pool: &PgPool, now: DateTime<Utc>, root: Option<&Path>) -> Router {
    let networks: Vec<IpNet> = vec!["10.10.0.0/16".parse().unwrap()];
    access::permit_networks(pool, &networks).await.unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let bus = Bus::new(16);
    let mut env = BTreeMap::from([("ANIMA_PUBLIC_ORIGIN".to_owned(), PUBLIC.to_owned())]);
    if let Some(root) = root {
        let root = root.to_str().unwrap().to_owned();
        env.insert("ANIMA_WEB_ROOT".to_owned(), root);
    }
    let edge = access::Edge::new(pool.clone(), bus.clone(), clock.clone());
    let dependencies = Dependencies {
        config: Arc::new(config::from_env(&env).unwrap()),
        ..dependencies(pool, clock, bus)
    };
    all_routes(dependencies, edge)
}

/// The database clock to the second: heartbeat partitions exist only near it.
async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(1)).unwrap()
}

/// A stored token of `kind` and `scope` made of 32 bytes of `fill`, valid until `expires_at`;
/// returns the text a client presents. Creates the operator first.
async fn stored(pool: &PgPool, token: Token, expires_at: DateTime<Utc>) -> String {
    stored_bytes(pool, token, [token.fill; 32], expires_at).await
}

/// As [`stored`], with the token's 32 bytes given.
async fn stored_bytes(
    pool: &PgPool,
    token: Token,
    bytes: [u8; 32],
    expires_at: DateTime<Utc>,
) -> String {
    let created: i64 = sqlx::query_scalar("select count(*) from access.operators")
        .fetch_one(pool)
        .await
        .unwrap();
    if created == 0 {
        set_operator(pool).await;
    }
    let text = URL_SAFE_NO_PAD.encode(bytes);
    let presented = match token.kind {
        "api_key" => format!("anima_{text}"),
        _ => text,
    };
    let (name, prefix) = match token.kind {
        "api_key" => (
            Some(format!("key {}", token.fill)),
            Some(presented[..12].to_owned()),
        ),
        _ => (None, None),
    };
    sqlx::query(
        "insert into access.api_tokens \
         (id, operator_id, token_hash, kind, name, display_prefix, scope, expires_at, created_at) \
         select gen_random_uuid(), id, $1, $2, $3, $4, $5, $6, $6 - interval '1 day' \
         from access.operators",
    )
    .bind(Sha256::digest(bytes).to_vec())
    .bind(token.kind)
    .bind(name)
    .bind(prefix)
    .bind(token.scope)
    .bind(expires_at)
    .execute(pool)
    .await
    .unwrap();
    presented
}

/// What [`stored`] makes: the kind and scope as `access.api_tokens` holds them, and the byte
/// the token is made of.
#[derive(Clone, Copy)]
struct Token {
    kind: &'static str,
    scope: &'static str,
    fill: u8,
}

const READ_KEY: Token = Token {
    kind: "api_key",
    scope: "read",
    fill: 1,
};
const WRITE_KEY: Token = Token {
    kind: "api_key",
    scope: "write",
    fill: 2,
};
const LOGIN: Token = Token {
    kind: "login",
    scope: "write",
    fill: 3,
};
const BROWSER: Token = Token {
    kind: "browser",
    scope: "write",
    fill: 4,
};
const BUILDER: Token = Token {
    kind: "builder",
    scope: "builder",
    fill: 5,
};

/// What the hub answered.
#[derive(Debug)]
struct Answer {
    status: StatusCode,
    content_type: Option<String>,
    challenge: bool,
    body: String,
}

/// Sends `method uri` with `headers` from `peer`.
async fn send(app: &Router, peer: &str, request: Request<Body>) -> Answer {
    let mut request = request;
    let peer: SocketAddr = peer.parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(peer));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let content_type = (response.headers().get(header::CONTENT_TYPE))
        .map(|value| value.to_str().unwrap().to_owned());
    let challenge = response.headers().contains_key(header::WWW_AUTHENTICATE);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Answer {
        status,
        content_type,
        challenge,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    }
}

/// `GET /metrics` with `headers`, from a permitted client.
async fn scrape(app: &Router, headers: &[(&str, &str)]) -> Answer {
    let mut request = Request::get("/metrics");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    send(app, PERMITTED, request.body(Body::empty()).unwrap()).await
}

/// `GET /metrics` with `Authorization: Bearer <token>`.
async fn scrape_with(app: &Router, token: &str) -> Answer {
    scrape(app, &[("authorization", &format!("Bearer {token}"))]).await
}

/// Asserts the K3 refusal: 401, no body, no `WWW-Authenticate`.
fn assert_refused(answer: &Answer, case: &str) {
    assert_eq!(
        answer.status,
        StatusCode::UNAUTHORIZED,
        "{case}: {answer:?}"
    );
    assert_eq!(answer.body, "", "{case}");
    assert!(!answer.challenge, "{case}");
}
