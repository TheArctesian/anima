//! The browser session of a hosted hub through the hub's router (hosted-mode.md HMD-061,
//! HMD-062): only the cloud's key makes a ticket, only a request from the public origin trades
//! it, once, for the cookie of the hub, and that cookie signs the browser in. A hub that is not
//! hosted has neither call. No ticket and no token reaches an entry (LOG-007).

use std::net::SocketAddr;

use anima_hub::{all_routes, fleet::access};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{HeaderMap, Request, header},
};
use http_body_util::BodyExt as _;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::control::{FakeLink, code, ok, written};
use super::hosted::{CLOUD_KEY, hosted_parts};
use crate::hub::{dependencies, start};

/// The default public origin of a hub: the address of the gateway.
const GATEWAY: &str = "https://hub.anima.ing";
const FROM_GATEWAY: [(&str, &str); 2] = [("origin", GATEWAY), ("host", "hub.anima.ing")];
const TICKET_CALL: &str =
    "mutation($k: String!) { hostedBrowserTicket(cloudKey: $k) { ticket expiresAt } }";
const SIGN_IN_CALL: &str =
    "mutation($t: String!) { hostedBrowserSignIn(ticket: $t) { expiresAt } }";

fn quiet_link() -> FakeLink {
    FakeLink::start(|_| ok(&json!([])))
}

/// A hosted hub with every route as `main.rs` serves it, behind the permitted networks of the
/// link mode and the guard of the host name: as the pool reaches it, from the loopback address
/// of its own container.
async fn hosted(pool: &PgPool, link: &FakeLink, folder: &tempfile::TempDir) -> Router {
    let parts = hosted_parts(pool, link, folder);
    access::permit_networks(pool, &parts.config.permitted_networks)
        .await
        .unwrap();
    let edge = access::Edge::new(pool.clone(), parts.bus.clone(), parts.clock.clone());
    all_routes(parts, edge)
}

/// Posts a GraphQL request from this computer with the extra `headers`; returns the headers of
/// the answer and its body, `Null` when the body is not JSON (a bare refusal).
async fn post(
    hub: &Router,
    headers: &[(&str, &str)],
    query: &str,
    variables: Value,
) -> (HeaderMap, Value) {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let mut request = Request::post("/graphql").header(header::CONTENT_TYPE, "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut request = request.body(Body::from(body)).unwrap();
    let peer: SocketAddr = "127.0.0.1:50000".parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(peer));
    let response = hub.clone().oneshot(request).await.unwrap();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        headers,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn ticket(hub: &Router) -> String {
    let (_, made) = post(
        hub,
        &[("host", "127.0.0.1:8443")],
        TICKET_CALL,
        json!({ "k": CLOUD_KEY }),
    )
    .await;
    assert!(made["errors"].is_null(), "{made}");
    let ends = (start() + chrono::TimeDelta::seconds(60)).to_rfc3339();
    assert_eq!(
        made["data"]["hostedBrowserTicket"]["expiresAt"],
        json!(ends)
    );
    let ticket = made["data"]["hostedBrowserTicket"]["ticket"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(ticket.len(), 43, "256 bits in base64url");
    ticket
}

/// The value of the `__Host-anima` cookie that the answer sets, with its attributes.
fn set_cookie(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::SET_COOKIE)?.to_str().ok()?;
    value.starts_with("__Host-anima=").then(|| value.to_owned())
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_061_the_gateway_trades_a_ticket_of_the_cloud_for_the_cookie_of_the_hub_once(
    pool: PgPool,
) {
    let (link, folder) = (quiet_link(), tempfile::tempdir().unwrap());
    let hub = hosted(&pool, &link, &folder).await;
    let ticket = ticket(&hub).await;

    let no_origin = post(
        &hub,
        &[("host", "hub.anima.ing")],
        SIGN_IN_CALL,
        json!({ "t": ticket }),
    )
    .await;
    assert!(set_cookie(&no_origin.0).is_none(), "no origin, no cookie");
    assert_eq!(code(&no_origin.1), "FORBIDDEN", "{}", no_origin.1);

    let (headers, answer) = post(&hub, &FROM_GATEWAY, SIGN_IN_CALL, json!({ "t": ticket })).await;
    assert!(answer["errors"].is_null(), "{answer}");
    let cookie = set_cookie(&headers).expect("the cookie of the hub");
    assert!(
        cookie.ends_with("; Max-Age=43200; Path=/; Secure; HttpOnly; SameSite=Strict"),
        "12 hours and the fixed attributes"
    );
    let token = cookie.split(';').next().unwrap().to_owned();

    let again = post(&hub, &FROM_GATEWAY, SIGN_IN_CALL, json!({ "t": ticket })).await;
    assert_eq!(code(&again.1), "UNAUTHENTICATED", "a ticket works once");
    assert!(set_cookie(&again.0).is_none());

    let signed_in = [
        ("origin", GATEWAY),
        ("host", "hub.anima.ing"),
        ("cookie", token.as_str()),
    ];
    let (_, me) = post(&hub, &signed_in, "{ me { username } }", json!({})).await;
    assert_eq!(me["data"]["me"]["username"], "owner", "{me}");

    let stored = written(&pool).await;
    assert!(
        stored.contains("A browser signed in with the Anima.ing account."),
        "{stored}"
    );
    let secret = token.trim_start_matches("__Host-anima=");
    assert!(!stored.contains(&ticket) && !stored.contains(secret));
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_061_a_wrong_key_makes_no_ticket_and_a_wrong_ticket_no_session(pool: PgPool) {
    let (link, folder) = (quiet_link(), tempfile::tempdir().unwrap());
    let hub = hosted(&pool, &link, &folder).await;
    let (_, wrong) = post(
        &hub,
        &[("host", "127.0.0.1:8443")],
        TICKET_CALL,
        json!({ "k": "not-the-key" }),
    )
    .await;
    assert_eq!(code(&wrong), "UNAUTHENTICATED", "{wrong}");
    let (_, guessed) = post(&hub, &FROM_GATEWAY, SIGN_IN_CALL, json!({ "t": "a-guess" })).await;
    assert_eq!(code(&guessed), "UNAUTHENTICATED", "{guessed}");
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_061_a_hub_that_is_not_hosted_has_no_browser_ticket_and_no_such_sign_in(pool: PgPool) {
    let clock: std::sync::Arc<dyn anima_hub::clock::Clock> =
        std::sync::Arc::new(anima_hub::clock::FixedClock(start()));
    let parts = dependencies(&pool, clock, anima_hub::fleet::events::Bus::new(16));
    let hub = crate::hub::Hub::serving(&pool, parts);
    let made = hub.post(None, TICKET_CALL, json!({ "k": CLOUD_KEY })).await;
    assert_eq!(code(&made), "FORBIDDEN", "{made}");
    let traded = hub.post(None, SIGN_IN_CALL, json!({ "t": "x" })).await;
    assert_eq!(code(&traded), "FORBIDDEN", "{traded}");
}

#[sqlx::test(migrations = "./migrations")]
async fn hmd_061_a_hosted_hub_refuses_a_request_with_a_foreign_host_or_origin(pool: PgPool) {
    let (link, folder) = (quiet_link(), tempfile::tempdir().unwrap());
    let hub = hosted(&pool, &link, &folder).await;
    for refused in [
        [
            ("origin", "https://evil.example"),
            ("host", "hub.anima.ing"),
        ],
        [("origin", GATEWAY), ("host", "evil.example")],
    ] {
        let (headers, answer) = post(&hub, &refused, SIGN_IN_CALL, json!({ "t": "x" })).await;
        assert_eq!(answer, Value::Null, "a bare 403 with no body");
        assert!(set_cookie(&headers).is_none());
    }
}
