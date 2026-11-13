//! The bearer layer, and agent secrets on every transport, on a pool that fails on use: an
//! answer of 401 or no identity proves no lookup ran, because a lookup on that pool fails (A-9,
//! R-092 D-1). The answers on a database are in the `it` crate (`tests/it/bearer/`).

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{TimeZone as _, Utc};
use http_body_util::BodyExt as _;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::{bearer_layer, covers};
use crate::{
    clock::{Clock, FixedClock},
    fleet::access::{
        Error, Scope, identify, identify_connection, identity::resolve, operator_for_token,
        tokens::Transport,
    },
};

/// A pool whose every use fails: nothing listens on port 1.
fn failing_pool() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("postgres://127.0.0.1:1/none")
        .expect("a lazy pool")
}

fn agent_token() -> String {
    format!("anima_agent_{}", URL_SAFE_NO_PAD.encode([9_u8; 32]))
}

fn enrolment_code() -> String {
    format!("anima_enrol_{}", URL_SAFE_NO_PAD.encode([9_u8; 16]))
}

/// A login token's spelling: what the hub would look up.
fn plain_token() -> String {
    URL_SAFE_NO_PAD.encode([3_u8; 32])
}

#[test]
fn int_003_a_read_scope_never_covers_a_write_scope() {
    assert!(covers(Scope::Read, Scope::Read));
    assert!(covers(Scope::Write, Scope::Read));
    assert!(covers(Scope::Write, Scope::Write));
    assert!(!covers(Scope::Read, Scope::Write));
}

#[test]
fn bld_022_the_builder_scope_covers_nothing_and_nothing_covers_it() {
    for other in [Scope::Read, Scope::Write] {
        assert!(!covers(Scope::Builder, other), "{other:?}");
        assert!(!covers(other, Scope::Builder), "{other:?}");
    }
    assert!(!covers(Scope::Builder, Scope::Builder));
}

fn bearer_headers(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {value}").parse().unwrap(),
    );
    headers
}

#[tokio::test]
async fn r_092_an_agent_secret_is_no_identity_and_never_looked_up() {
    let db = failing_pool();
    let now = Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap();
    // The control: a token that is looked up fails on this pool.
    let looked_up = resolve(&db, &plain_token(), Transport::Bearer, now).await;
    assert!(matches!(looked_up, Err(Error::Unreachable(_))));

    for secret in [agent_token(), enrolment_code()] {
        for transport in [Transport::Bearer, Transport::Cookie] {
            let resolved = resolve(&db, &secret, transport, now).await;
            assert!(matches!(resolved, Ok(None)), "{transport:?}");
        }
        let header = identify(&db, &bearer_headers(&secret), None, now).await;
        assert!(matches!(header, Ok(None)));
        let payload = json!({ "authorization": format!("Bearer {secret}") });
        let socket = identify_connection(&db, &payload, now).await;
        assert!(matches!(socket, Ok(None)));
        let tunnel = operator_for_token(&db, &secret, now).await;
        assert!(matches!(tunnel, Err(Error::Unauthenticated)));
    }
}

/// `/metrics` behind the layer at `scope`, on a pool that fails on use; counts the requests
/// that reached the route.
fn guarded(scope: Scope) -> (Router, Arc<AtomicUsize>) {
    let reached = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&reached);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(
        Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap(),
    ));
    let route = get(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        async { "reached" }
    });
    let router = Router::new()
        .route("/metrics", route)
        .route_layer(bearer_layer(failing_pool(), clock, scope));
    (router, reached)
}

/// The status, whether a `WWW-Authenticate` header came, and the body.
async fn send(router: &Router, headers: &[(&str, &str)]) -> (StatusCode, bool, Vec<u8>) {
    let mut request = Request::get("/metrics");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let challenge = response.headers().contains_key(header::WWW_AUTHENTICATE);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, challenge, body.to_vec())
}

#[tokio::test]
async fn a_9_a_request_without_one_bearer_token_gets_401_with_no_lookup_and_no_route() {
    let (router, reached) = guarded(Scope::Read);
    let token = plain_token();
    let bearer = format!("Bearer {token}");
    let agent = format!("Bearer {}", agent_token());
    let enrolment = format!("Bearer {}", enrolment_code());
    let cookie = format!("__Host-anima={token}");
    let cases: [&[(&str, &str)]; 11] = [
        &[],
        &[("cookie", &cookie), ("origin", "https://hub.anima.ing")],
        &[("authorization", &bearer), ("authorization", &bearer)],
        &[("authorization", &format!("Basic {token}"))],
        &[("authorization", &token)],
        &[("authorization", "Bearer")],
        &[("authorization", "Bearer ")],
        &[("authorization", "Bearer not-a-token")],
        &[("authorization", &format!("Bearer {token}x"))],
        &[("authorization", &agent)],
        &[("authorization", &enrolment)],
    ];
    for headers in cases {
        let answer = send(&router, headers).await;
        assert_eq!(
            answer,
            (StatusCode::UNAUTHORIZED, false, Vec::new()),
            "{headers:?}"
        );
    }
    assert_eq!(reached.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_9_a_token_that_cannot_be_looked_up_gets_503_with_an_empty_body() {
    let (router, reached) = guarded(Scope::Read);
    let bearer = format!("Bearer {}", plain_token());
    let answer = send(&router, &[("authorization", &bearer)]).await;
    assert_eq!(answer, (StatusCode::SERVICE_UNAVAILABLE, false, Vec::new()));
    assert_eq!(reached.load(Ordering::SeqCst), 0);
}
