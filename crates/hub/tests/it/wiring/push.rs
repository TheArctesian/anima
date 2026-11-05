//! WP-09 handoff 3: `/push/{token}` as `main.rs` mounts it, inside the request log and behind
//! the permitted-networks layer (NET-002, LOG-007, R-028).

use std::{net::SocketAddr, sync::Arc};

use anima_hub::{
    access,
    checks::push_router,
    clock::{Clock, FixedClock},
    events::{Bus, process_log},
    logged, router,
    schema::build_schema,
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use chrono::{DateTime, Utc};
use ipnet::IpNet;
use sqlx::PgPool;
use tower::ServiceExt as _;

use super::{PVE_01, uri_log::Capture};
use crate::hub::dependencies;

/// 32 bytes in unpadded base64url, as a push token is.
const TOKEN: &str = "WIREDpushTOKENwiredPUSHtokenWIREDpushTOKENw";
const PERMITTED: &str = "10.10.0.5:40000";
const OUTSIDE: &str = "192.0.2.9:40000";

/// The hub's routes as `main.rs` composes them, at `now`.
fn app(pool: &PgPool, now: DateTime<Utc>) -> Router {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let bus = Bus::new(16);
    let schema = build_schema(dependencies(pool, clock.clone(), bus.clone()));
    let push = logged(push_router(pool.clone(), bus.clone(), clock.clone()));
    let edge = access::Edge::new(pool.clone(), bus, clock.clone());
    router(schema, pool.clone(), clock)
        .merge(push)
        .layer(access::permitted_networks_layer(edge))
}

async fn push_check(pool: &PgPool) {
    sqlx::query(
        "insert into checks.checks (id, device_id, type, name, push_token) \
         values (gen_random_uuid(), $1::uuid, 'push', 'Wired push', $2)",
    )
    .bind(PVE_01)
    .bind(TOKEN)
    .execute(pool)
    .await
    .unwrap();
}

async fn push_from(app: &Router, peer: &str) -> StatusCode {
    let mut request = Request::get(format!("/push/{TOKEN}?status=up"))
        .body(Body::empty())
        .unwrap();
    let peer: SocketAddr = peer.parse().unwrap();
    request.extensions_mut().insert(ConnectInfo(peer));
    app.clone().oneshot(request).await.unwrap().status()
}

async fn beats(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "select count(*) from checks.heartbeats h join checks.checks c on c.id = h.check_id \
         where c.push_token = $1",
    )
    .bind(TOKEN)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn h_3_a_push_is_answered_inside_the_permitted_networks_and_logged_without_its_token(
    pool: PgPool,
) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in uri_log: a second dispatcher keeps another test thread from caching this test's
    // callsites as disabled.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let networks: Vec<IpNet> = vec!["10.10.0.0/16".parse().unwrap()];
    access::permit_networks(&pool, &networks).await.unwrap();
    push_check(&pool).await;
    let now = sqlx::query_scalar("select now()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let app = app(&pool, now);

    assert_eq!(push_from(&app, OUTSIDE).await, StatusCode::FORBIDDEN);
    assert_eq!(beats(&pool).await, 0);
    assert_eq!(push_from(&app, PERMITTED).await, StatusCode::OK);
    assert_eq!(beats(&pool).await, 1);

    let logged = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(logged.contains("/push/[redacted]"), "logged: {logged}");
    assert!(!logged.contains(TOKEN), "logged: {logged}");
}
