//! `GET /metrics` takes a READ bearer token or better and refuses everything else with an empty
//! 401 before the exposition runs (INT-001, INT-003, D-24, NET-002, WP-43 A-9).

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::{
    access::{self, Scope},
    clock::{Clock, FixedClock},
    events::process_log,
    exposition,
};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use chrono::TimeDelta;
use serde_json::Value;
use sqlx::PgPool;

use super::{
    Answer, BROWSER, INDEX_HTML, LOGIN, PERMITTED, PUBLIC, READ_KEY, WRITE_KEY, app, app_at,
    assert_refused, database_now, scrape, scrape_with, send, stored,
};

/// The Prometheus text format 0.0.4 (INT-002).
const PROMETHEUS: &str = "text/plain; version=0.0.4; charset=utf-8";
/// On `pve-01` in the fixtures; its stored status is unknown until a test sets it.
const PVE_01_PING: &str = "0190a000-0000-7000-8003-000000000001";

/// The value of `anima_check_status` for `check` in an exposition.
fn check_status(text: &str, check: &str) -> Option<String> {
    let label = format!("check_id=\"{check}\"");
    text.lines()
        .find(|line| line.starts_with("anima_check_status{") && line.contains(&label))
        .map(|line| line.rsplit_once(' ').unwrap().1.to_owned())
}

async fn set_status(pool: &PgPool, check: &str, status: &str) {
    sqlx::query("UPDATE checks.checks SET status = $2 WHERE id = $1::uuid")
        .bind(check)
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn d_24_int_001_metrics_without_a_token_answers_401_and_starts_no_build(pool: PgPool) {
    let now = database_now(&pool).await;
    let key = stored(&pool, READ_KEY, now + TimeDelta::days(1)).await;
    let (app, _web) = app(&pool, now).await;

    assert_refused(&scrape(&app, &[]).await, "no token");
    // A build started by the refused scrape would serve this second for ten seconds, with
    // the status as it was then.
    set_status(&pool, PVE_01_PING, "down").await;
    let first = scrape_with(&app, &key).await;

    assert_eq!(first.status, StatusCode::OK, "{first:?}");
    assert_eq!(check_status(&first.body, PVE_01_PING).as_deref(), Some("0"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn int_001_a_read_key_a_write_key_and_a_login_token_scrape_prometheus_text(pool: PgPool) {
    let now = database_now(&pool).await;
    let until = now + TimeDelta::seconds(1);
    let (app, _web) = app(&pool, now).await;
    for token in [READ_KEY, WRITE_KEY, LOGIN] {
        let presented = stored(&pool, token, until).await;
        let answer = scrape_with(&app, &presented).await;
        assert_eq!(answer.status, StatusCode::OK, "{}: {answer:?}", token.kind);
        assert_eq!(answer.content_type.as_deref(), Some(PROMETHEUS));
        assert!(answer.body.contains("anima_check_status{"), "{answer:?}");
    }
}

/// `{ me { username } }` on `/graphql` with `headers`.
async fn me(app: &Router, headers: &[(&str, &str)]) -> Value {
    let mut request = Request::post("/graphql").header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let body = Body::from(r#"{"query":"{ me { username } }"}"#);
    let answer = send(app, PERMITTED, request.body(body).unwrap()).await;
    serde_json::from_str(&answer.body).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn int_003_a_9_every_other_bearer_and_the_browser_cookie_get_an_empty_401(pool: PgPool) {
    let now = database_now(&pool).await;
    let key = stored(&pool, READ_KEY, now + TimeDelta::days(1)).await;
    let browser = stored(&pool, BROWSER, now + TimeDelta::days(1)).await;
    let mut expired = READ_KEY;
    expired.fill = 7;
    let expired = stored(&pool, expired, now - TimeDelta::seconds(1)).await;
    let mut at_expiry = WRITE_KEY;
    at_expiry.fill = 8;
    let at_expiry = stored(&pool, at_expiry, now).await;
    let (app, _web) = app(&pool, now).await;
    let cookie = [
        ("cookie", format!("__Host-anima={browser}")),
        ("origin", PUBLIC.to_owned()),
    ];
    let cookie: Vec<(&str, &str)> = cookie.iter().map(|(n, v)| (*n, v.as_str())).collect();
    // The control: the cookie signs the browser in on /graphql.
    assert_eq!(
        me(&app, &cookie).await["data"]["me"]["username"],
        "operator"
    );
    assert_refused(&scrape(&app, &cookie).await, "browser cookie");

    let bearer = format!("Bearer {key}");
    let basic = format!("Basic {key}");
    let unknown = format!("anima_{}", "A".repeat(43));
    let headers: [(&str, &[(&str, &str)]); 6] = [
        (
            "two headers",
            &[("authorization", &bearer), ("authorization", &bearer)],
        ),
        ("basic", &[("authorization", &basic)]),
        ("no scheme", &[("authorization", &key)]),
        ("no token", &[("authorization", "Bearer")]),
        ("x-header", &[("x-authorization", &bearer)]),
        ("none", &[]),
    ];
    for (case, headers) in headers {
        assert_refused(&scrape(&app, headers).await, case);
    }
    for (case, token) in [
        ("browser token as bearer", browser.as_str()),
        ("expired key", &expired),
        ("key at its expiry", &at_expiry),
        ("unknown key", &unknown),
        ("malformed", "anima_not-a-key"),
    ] {
        assert_refused(&scrape_with(&app, token).await, case);
    }
    assert_eq!(scrape_with(&app, &key).await.status, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn net_002_the_bearer_check_covers_metrics_and_nothing_else(pool: PgPool) {
    let now = database_now(&pool).await;
    let key = stored(&pool, READ_KEY, now + TimeDelta::days(1)).await;
    let get = |uri: &str| Request::get(uri).body(Body::empty()).unwrap();
    let (app, _web) = app(&pool, now).await;
    let page = send(&app, PERMITTED, get("/devices/pve-01")).await;
    assert_eq!(
        (page.status, page.body.as_str()),
        (StatusCode::OK, INDEX_HTML)
    );
    assert_refused(&scrape(&app, &[]).await, "with a web root");

    // Without a web root, the hub's fallback stays the default 404. `all_routes` merges the
    // empty web router after `/metrics`, so its default fallback is the one kept here.
    let app = app_at(&pool, now, None).await;

    let health = send(&app, PERMITTED, get("/health")).await;
    assert_eq!(
        (health.status, health.body.as_str()),
        (StatusCode::OK, "ok")
    );
    let unknown = send(&app, PERMITTED, get("/nothing-here")).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND, "{unknown:?}");
    let signed_out = me(&app, &[]).await;
    assert_eq!(
        signed_out["errors"][0]["extensions"]["code"],
        "UNAUTHENTICATED"
    );

    let bearer = format!("Bearer {key}");
    let request = Request::get("/metrics").header("authorization", &bearer);
    let outside = send(
        &app,
        "192.0.2.9:40000",
        request.body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(outside.status, StatusCode::FORBIDDEN, "{outside:?}");

    // Merged last, `/metrics` gives the router its fallback: the bearer check must not wrap
    // it, or every unknown path would answer 401 (axum keeps the last merged default fallback).
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let last = Router::new().merge(exposition(pool.clone(), clock));
    let unknown = send(&last, PERMITTED, get("/nothing-here")).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND, "{unknown:?}");
    assert_refused(&scrape(&last, &[]).await, "merged last");
}

#[sqlx::test(migrations = "./migrations")]
async fn int_003_a_read_key_is_refused_where_write_is_asked(pool: PgPool) {
    let now = database_now(&pool).await;
    let until = now + TimeDelta::days(1);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now));
    let app = Router::new()
        .route("/metrics", get(|| async { "written" }))
        .route_layer(access::bearer_layer(pool.clone(), clock, Scope::Write));
    let read = stored(&pool, READ_KEY, until).await;
    assert_refused(&scrape_with(&app, &read).await, "read key");
    for token in [WRITE_KEY, LOGIN] {
        let presented = stored(&pool, token, until).await;
        let answer: Answer = scrape_with(&app, &presented).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", token.kind);
    }
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

#[sqlx::test(migrations = "./migrations")]
async fn log_007_a_scrape_is_logged_by_its_route_never_its_token(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in wiring::uri_log: a second dispatcher keeps another test thread from caching this
    // test's callsites as disabled.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let now = database_now(&pool).await;
    let key = stored(&pool, READ_KEY, now + TimeDelta::days(1)).await;
    let mut expired = WRITE_KEY;
    expired.fill = 9;
    let expired = stored(&pool, expired, now).await;
    let (app, _web) = super::app(&pool, now).await;

    assert_eq!(scrape_with(&app, &key).await.status, StatusCode::OK);
    assert_refused(&scrape_with(&app, &expired).await, "expired key");

    let log = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("/metrics"), "{log}");
    for secret in [
        &key,
        &expired,
        &key["anima_".len()..],
        &expired["anima_".len()..],
    ] {
        assert!(!log.contains(secret), "{log}");
    }
}
