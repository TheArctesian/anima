//! `GET /metrics` against the database: the K7 series and values, the labels, the ten-second
//! cache and the fixed statement count (INT-001, INT-002, D-24). The router does no
//! authentication; the mount behind the bearer layer is tested with the wiring.

mod cache;
mod cost;
mod labels;
mod series;

use std::sync::{Arc, Mutex};

use anima_hub::{
    checks::{self, Beat, CheckState, Recorded, exposition_router},
    clock::Clock,
    ids::CheckId,
    status::Status,
};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use http_body_util::BodyExt as _;
use sqlx::PgPool;
use tower::ServiceExt as _;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const OLLAMA: &str = "0190a000-0000-7000-8002-000000000003";
const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";
const PVE_01_PING: &str = "0190a000-0000-7000-8003-000000000001";
const PVE_01_SSH: &str = "0190a000-0000-7000-8003-000000000002";
/// `https://pve-01.home.lan:8006/`.
const PVE_01_WEB: &str = "0190a000-0000-7000-8003-00000000001a";
const DOCKER_PING: &str = "0190a000-0000-7000-8003-000000000003";
const OLLAMA_PING: &str = "0190a000-0000-7000-8003-000000000005";
const OLLAMA_SSH: &str = "0190a000-0000-7000-8003-000000000006";
/// `http://ha.home.lan:8123/`: plain HTTP, so never a certificate.
const HA_WEB: &str = "0190a000-0000-7000-8003-000000000019";
/// On `jellyfin`, whose manual maintenance flag is on in the fixtures.
const JELLYFIN_PING: &str = "0190a000-0000-7000-8003-00000000000a";
const HOME_SITE: &str = "0190a000-0000-7000-8001-000000000001";

/// A clock the test sets.
struct SetClock(Mutex<DateTime<Utc>>);

impl SetClock {
    fn at(now: DateTime<Utc>) -> Arc<Self> {
        Arc::new(Self(Mutex::new(now)))
    }

    fn advance(&self, by: TimeDelta) {
        let mut now = self.0.lock().unwrap();
        *now += by;
    }
}

impl Clock for SetClock {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

/// The database clock to the second: heartbeat partitions exist only near it.
async fn database_now(pool: &PgPool) -> DateTime<Utc> {
    let now: DateTime<Utc> = sqlx::query_scalar("select pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    now.duration_trunc(TimeDelta::seconds(1)).unwrap()
}

/// The router on `pool` with a clock the test moves.
fn rig(pool: &PgPool, now: DateTime<Utc>) -> (Router, Arc<SetClock>) {
    let clock = SetClock::at(now);
    let shared: Arc<dyn Clock> = clock.clone();
    (exposition_router(pool.clone(), shared), clock)
}

/// One answer of the route.
struct Scrape {
    status: StatusCode,
    content_type: Option<String>,
    body: String,
}

async fn scrape(router: &Router) -> Scrape {
    let request = Request::get("/metrics").body(Body::empty()).unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap().to_owned());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Scrape {
        status,
        content_type,
        body: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

/// The body of a successful scrape.
async fn body(router: &Router) -> String {
    let scraped = scrape(router).await;
    assert_eq!(scraped.status, StatusCode::OK, "{}", scraped.body);
    scraped.body
}

/// The value of the one sample of `family` whose labels hold `label="value"`, or `None`.
fn value(text: &str, family: &str, labels: &[(&str, &str)]) -> Option<String> {
    let found: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with(&format!("{family}{{")))
        .filter(|line| {
            labels
                .iter()
                .all(|(name, value)| line.contains(&format!("{name}=\"{value}\"")))
        })
        .collect();
    assert!(found.len() <= 1, "{found:?}");
    found
        .first()
        .map(|line| line.rsplit_once(' ').unwrap().1.to_owned())
}

/// The value of the check's sample in `family`.
fn of_check(text: &str, family: &str, check: &str) -> Option<String> {
    value(text, family, &[("check_id", check)])
}

/// Sets the stored status of every check, then of the listed ones.
async fn set_statuses(pool: &PgPool, all: &str, each: &[(&str, &str)]) {
    sqlx::query("UPDATE checks.checks SET status = $1")
        .bind(all)
        .execute(pool)
        .await
        .unwrap();
    for (check, status) in each {
        sqlx::query("UPDATE checks.checks SET status = $2 WHERE id = $1::uuid")
            .bind(check)
            .bind(status)
            .execute(pool)
            .await
            .unwrap();
    }
}

/// Records a heartbeat for the check at `at`.
async fn record(pool: &PgPool, check: &str, at: DateTime<Utc>, response_time_ms: Option<u32>) {
    sqlx::query("select checks.ensure_day_partition($1)")
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
    let beat = Beat {
        at,
        status: Status::Up,
        response_time_ms,
        message: None,
        state: CheckState {
            status: Status::Up,
            failures: 0,
            next_due_at: None,
        },
        resolved_ip: None,
        cert_not_after: None,
        domain_not_after: None,
    };
    let id: CheckId = check.parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let recorded = checks::record_beat(&mut conn, id, &beat).await.unwrap();
    assert_eq!(recorded, Recorded::Written);
}
