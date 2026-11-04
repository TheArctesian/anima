//! The NET-002 layer in front of the router, as `main.rs` mounts it: refusals before any
//! GraphQL work with one `ACCESS` warning event a minute per address, and the `ClientAddr`
//! of every request that passes (NET-001, NET-002, D-10, D-11).

mod permit;
mod untrusted;

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use anima_hub::{
    access::{self, ClientAddr},
    clock::Clock,
    events::{Bus, Signal},
    router,
    schema::build_schema,
};
use axum::{
    Extension, Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
    routing::get,
};
use chrono::{DateTime, TimeDelta, Utc};
use http_body_util::BodyExt as _;
use ipnet::IpNet;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::hub::{dependencies, start};

/// A clock the test moves forward.
struct StepClock(Mutex<DateTime<Utc>>);

impl Clock for StepClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

impl StepClock {
    fn advance(&self, by: TimeDelta) {
        *self.0.lock().unwrap() += by;
    }
}

struct Edge {
    app: Router,
    clock: Arc<StepClock>,
    bus: Bus,
}

/// The hub router behind the layer, plus `/client` answering with the request's `ClientAddr`.
async fn edge(pool: &PgPool, permitted: &[&str]) -> Edge {
    permit(pool, permitted).await;
    let clock = Arc::new(StepClock(Mutex::new(start())));
    let dyn_clock: Arc<dyn Clock> = clock.clone();
    let bus = Bus::new(16);
    let schema = build_schema(dependencies(pool, dyn_clock.clone(), bus.clone()));
    let app =
        router(schema, pool.clone(), dyn_clock.clone())
            .route(
                "/client",
                get(
                    |Extension(ClientAddr(client)): Extension<ClientAddr>| async move {
                        client.to_string()
                    },
                ),
            )
            .layer(access::permitted_networks_layer(
                pool.clone(),
                bus.clone(),
                dyn_clock,
            ));
    Edge { app, clock, bus }
}

async fn permit(pool: &PgPool, permitted: &[&str]) {
    let networks: Vec<IpNet> = permitted.iter().map(|net| net.parse().unwrap()).collect();
    access::permit_networks(pool, &networks).await.unwrap();
}

fn typename() -> Request<Body> {
    Request::post("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "query": "{ __typename }" }).to_string()))
        .unwrap()
}

fn get_path(path: &str) -> Request<Body> {
    Request::get(path).body(Body::empty()).unwrap()
}

impl Edge {
    /// Sends `request` from TCP peer `peer`; returns the status and the body text.
    async fn send_from(&self, peer: &str, mut request: Request<Body>) -> (StatusCode, String) {
        let peer: SocketAddr = peer.parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        self.send(request).await
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, String) {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// `(severity, source, message)` of every stored event, oldest first.
async fn events(pool: &PgPool) -> Vec<(String, String, String)> {
    sqlx::query_as("select severity, source, message from events.event_log order by at, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

fn assert_forbidden((status, body): &(StatusCode, String)) {
    assert_eq!(*status, StatusCode::FORBIDDEN, "{body}");
    let body: Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        body["errors"][0]["extensions"]["code"], "FORBIDDEN",
        "{body}"
    );
    assert_eq!(
        body["errors"][0]["message"],
        "This address is not in a permitted network."
    );
}

fn refusal_of(client: &str) -> (String, String, String) {
    (
        "warning".to_owned(),
        "access".to_owned(),
        format!("Refused a request from {client}: the address is not in a permitted network."),
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_an_address_outside_every_network_gets_403_and_one_access_warning(pool: PgPool) {
    let edge = edge(&pool, &["100.64.0.0/10"]).await;
    let mut signals = edge.bus.subscribe();
    assert_forbidden(&edge.send_from("203.0.113.9:40000", typename()).await);
    assert_eq!(events(&pool).await, vec![refusal_of("203.0.113.9")]);
    let Ok(Signal::Event(published)) = signals.try_recv() else {
        panic!("the refusal is published");
    };
    assert_eq!(published.message, refusal_of("203.0.113.9").2);
    assert_forbidden(
        &edge
            .send_from("203.0.113.9:40001", get_path("/nowhere"))
            .await,
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_11_refusal_events_are_limited_to_one_a_minute_per_address(pool: PgPool) {
    let edge = edge(&pool, &["100.64.0.0/10"]).await;
    for _ in 0..20 {
        assert_forbidden(&edge.send_from("203.0.113.9:40000", typename()).await);
    }
    assert_forbidden(&edge.send_from("[2001:db8::7]:40000", typename()).await);
    edge.clock.advance(TimeDelta::seconds(59));
    assert_forbidden(&edge.send_from("203.0.113.9:40000", typename()).await);
    assert_eq!(
        events(&pool).await,
        vec![refusal_of("203.0.113.9"), refusal_of("2001:db8::7")]
    );
    edge.clock.advance(TimeDelta::seconds(1));
    assert_forbidden(&edge.send_from("203.0.113.9:40000", typename()).await);
    assert_eq!(
        events(&pool).await.len(),
        3,
        "a minute later it is written again"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_inside_a_network_passes_with_its_client_address(pool: PgPool) {
    let edge = edge(&pool, &["100.64.0.0/10", "fd7a:115c:a1e0::/48"]).await;
    let (status, body) = edge.send_from("100.64.0.7:40000", typename()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, r#"{"data":{"__typename":"Query"}}"#);
    let v4 = edge
        .send_from("100.64.0.7:40000", get_path("/client"))
        .await;
    assert_eq!(v4, (StatusCode::OK, "100.64.0.7".to_owned()));
    let v6 = edge
        .send_from("[fd7a:115c:a1e0::5]:40000", get_path("/client"))
        .await;
    assert_eq!(v6, (StatusCode::OK, "fd7a:115c:a1e0::5".to_owned()));
    assert_forbidden(
        &edge
            .send_from("[fd7a:115c:a1e1::5]:40000", get_path("/client"))
            .await,
    );
    assert_eq!(events(&pool).await, vec![refusal_of("fd7a:115c:a1e1::5")]);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_11_an_ipv4_mapped_peer_is_checked_and_passed_on_as_ipv4(pool: PgPool) {
    let edge = edge(&pool, &["100.64.0.0/10"]).await;
    let inside = edge
        .send_from("[::ffff:100.64.0.7]:40000", get_path("/client"))
        .await;
    assert_eq!(inside, (StatusCode::OK, "100.64.0.7".to_owned()));
    assert_forbidden(
        &edge
            .send_from("[::ffff:8.8.8.8]:40000", get_path("/client"))
            .await,
    );
    assert_eq!(events(&pool).await, vec![refusal_of("8.8.8.8")]);
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_a_refused_login_never_reaches_graphql(pool: PgPool) {
    crate::hub::set_operator(&pool).await;
    let edge = edge(&pool, &["100.64.0.0/10"]).await;
    let query = "mutation($i: LoginInput!) { login(input: $i) { token } }";
    let input =
        json!({ "i": { "username": crate::hub::USERNAME, "password": crate::hub::PASSWORD } });
    let login = Request::post("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "query": query, "variables": input }).to_string(),
        ))
        .unwrap();
    let refused = edge.send_from("192.0.2.1:40000", login).await;
    assert_forbidden(&refused);
    assert!(!refused.1.contains("token\":\""), "{}", refused.1);
    let attempts: i64 = sqlx::query_scalar("select count(*) from access.login_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_without_a_peer_address_the_request_is_refused(pool: PgPool) {
    let edge = edge(&pool, &["0.0.0.0/0", "::/0"]).await;
    let (status, body) = edge.send(get_path("/client")).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert!(!body.contains("0.0.0.0"), "{body}");
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_an_unreadable_list_refuses_the_request(pool: PgPool) {
    let edge = edge(&pool, &["100.64.0.0/10"]).await;
    pool.close().await;
    let (status, body) = edge
        .send_from("100.64.0.7:40000", get_path("/client"))
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["errors"][0]["extensions"]["code"], "UNAVAILABLE");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_10_a_changed_list_is_used_after_30_seconds(pool: PgPool) {
    let edge = edge(&pool, &["10.0.0.0/8"]).await;
    assert_forbidden(
        &edge
            .send_from("100.64.0.7:40000", get_path("/client"))
            .await,
    );
    permit(&pool, &["100.64.0.0/10"]).await;
    edge.clock.advance(TimeDelta::seconds(29));
    assert_forbidden(
        &edge
            .send_from("100.64.0.7:40000", get_path("/client"))
            .await,
    );
    edge.clock.advance(TimeDelta::seconds(1));
    let passed = edge
        .send_from("100.64.0.7:40000", get_path("/client"))
        .await;
    assert_eq!(passed, (StatusCode::OK, "100.64.0.7".to_owned()));
    assert_forbidden(&edge.send_from("10.0.0.1:40000", get_path("/client")).await);
}
