//! How the enrol route answers when it cannot finish: past its deadline it is `UNAVAILABLE`
//! and the code it marked used stays used (MET-012, K8 Section 3); without the
//! permitted-networks layer it serves nothing (NET-002).

use std::{sync::Arc, time::Duration};

use agent_core::wire::EnrolmentCode;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, Utc};
use http_body_util::BodyExt as _;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::{
    clock::{Clock, FixedClock},
    events::{Bus, ClientAddr},
    metrics::ingest::{make_code, routes::agent_router_within},
};

use super::{database_now, enrolment::waits_for_locks, pve};

fn router(pool: &PgPool, now: DateTime<Utc>, answer_time: Duration) -> Router {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    agent_router_within(pool.clone(), Bus::new(16), clock, answer_time)
}

/// Posts the code to `/agent/v1/enrol`, from a permitted address when `addressed`.
async fn enrol(router: &Router, code: &str, addressed: bool) -> (StatusCode, String) {
    let body = json!({ "code": code }).to_string();
    let mut request = Request::post("/agent/v1/enrol")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    if addressed {
        let client = ClientAddr("100.64.0.7".parse().unwrap());
        request.extensions_mut().insert(client);
    }
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// A stored code for pve-01, made at `now`; its text.
async fn code_for_pve(pool: &PgPool, now: DateTime<Utc>) -> String {
    let code = EnrolmentCode::from_random([3; 16]);
    let text = code.expose().to_owned();
    let mut conn = pool.acquire().await.unwrap();
    make_code(&mut conn, pve(), code, now).await.unwrap();
    text
}

async fn agents(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM metrics.agents")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_012_a_code_stays_used_when_its_exchange_failed_after_the_mark(pool: PgPool) {
    let now = database_now(&pool).await;
    let code = code_for_pve(&pool, now).await;
    // A slow writer holds the device row, so the exchange's second step waits for it. Its
    // deadline passes only once it waits, after the code was marked: the wait is seen in
    // pg_stat_activity, not guessed from a short deadline, so a loaded machine cannot turn it
    // into another outcome.
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM devices.devices WHERE id = $1 FOR UPDATE")
        .bind(pve())
        .execute(&mut *holder)
        .await
        .unwrap();

    let deadline = router(&pool, now, Duration::from_secs(3));
    let first = tokio::spawn({
        let code = code.clone();
        async move { enrol(&deadline, &code, true).await }
    });
    assert!(
        waits_for_locks(&pool, &first, 1).await,
        "the exchange waits"
    );
    assert_eq!(
        first.await.unwrap(),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            r#"{"error":"UNAVAILABLE"}"#.to_owned()
        )
    );
    holder.rollback().await.unwrap();

    let router = router(&pool, now, Duration::from_secs(30));
    let second = enrol(&router, &code, true).await;
    assert_eq!(
        second,
        (
            StatusCode::UNAUTHORIZED,
            r#"{"error":"ENROLMENT_REFUSED"}"#.to_owned()
        )
    );
    assert_eq!(agents(&pool).await, 0);
    let source: String =
        sqlx::query_scalar("SELECT metrics_source FROM devices.devices WHERE id = $1")
            .bind(pve())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(source, "none");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn net_002_a_route_without_the_permitted_networks_layer_serves_nothing(pool: PgPool) {
    let now = database_now(&pool).await;
    let code = code_for_pve(&pool, now).await;
    let router = router(&pool, now, Duration::from_secs(10));

    let (status, body) = enrol(&router, &code, false).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, r#"{"error":"UNAVAILABLE"}"#);
    // The code was not touched, so it still works behind the layer.
    assert_eq!(enrol(&router, &code, true).await.0, StatusCode::OK);
}
