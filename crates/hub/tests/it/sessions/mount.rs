//! `/tunnel` as `main.rs` mounts it: logged and behind the permitted-networks layer, so a
//! peer outside every permitted network never reaches the ticket, even with a valid one from
//! the address it was issued to (K3, NET-002, R-051).

use std::{net::SocketAddr, sync::Arc};

use anima_hub::{
    access,
    clock::{Clock, FixedClock},
    events::Bus,
    logged,
    sessions::{Live, tunnel_router},
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt as _;
use ipnet::IpNet;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::{
    PVE_01,
    harness::{CLIENT_ADDR, schema_at, vault},
    now,
    starting::{ready, redeem, started},
};

/// The tunnel route wrapped as `main.rs` wraps it, permitting only `permitted`.
async fn mounted(pool: &PgPool, live: &Live, permitted: &str) -> Router {
    let networks: Vec<IpNet> = vec![permitted.parse().unwrap()];
    access::permit_networks(pool, &networks).await.unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let bus = Bus::new(16);
    // No guacd listens on port 1; no request in these tests gets past the ticket.
    let tunnel = tunnel_router(
        pool.clone(),
        vault(),
        bus.clone(),
        clock.clone(),
        "127.0.0.1:1".to_owned(),
        live.clone(),
    );
    logged(tunnel).layer(access::permitted_networks_layer(access::Edge::new(
        pool.clone(),
        bus,
        clock,
    )))
}

/// Opens `/tunnel?token=<token>` from `peer`; returns the status and the body text.
async fn open_from(app: &Router, peer: &str, token: &str) -> (StatusCode, String) {
    let mut request = Request::get(format!("/tunnel?token={token}"))
        .body(Body::empty())
        .unwrap();
    let peer: SocketAddr = format!("{peer}:50000").parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(peer));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn net_002_a_refused_peer_cannot_open_the_tunnel_with_a_valid_ticket(pool: PgPool) {
    let live = Live::default();
    ready(&pool, &[PVE_01]).await;
    let ticket = started(&schema_at(&pool, now(), &live), PVE_01).await;
    let app = mounted(&pool, &live, "192.168.0.0/16").await;

    let (status, body) = open_from(&app, CLIENT_ADDR, ticket["token"].as_str().unwrap()).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        body["errors"][0]["message"],
        "This address is not in a permitted network."
    );
    let refusals: Vec<(String,)> =
        sqlx::query_as("select message from events.event_log where source = 'access'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        redeem(&live, &ticket, now()).is_some(),
        "the refused request never reached the ticket"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_a_permitted_peer_reaches_the_tunnel_route(pool: PgPool) {
    let app = mounted(&pool, &Live::default(), "10.10.0.0/16").await;

    let (status, body) = open_from(&app, CLIENT_ADDR, "unknown").await;

    // The route's own refusal of an unknown ticket: 403 with no body (K4 Section 2).
    assert_eq!((status, body.as_str()), (StatusCode::FORBIDDEN, ""));
}
