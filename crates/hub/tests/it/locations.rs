//! Location samples and the context of a transfer event through the hub (K-E4 Sections 11 to
//! 13; R-145, R-146, LOG-007).

use std::{net::SocketAddr, sync::Arc};

use anima_hub::{
    clock::{Clock, FixedClock},
    engine::{files::Store, sealing_keys::SealingKeys},
    fleet::events::Bus,
    router,
    schema::{Dependencies, build_schema},
};
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use http_body_util::BodyExt as _;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt as _;

mod context;
mod dawarich;
mod owntracks;

use crate::hub::{CLIENT, Hub, dependencies, error, set_operator, start};

const SAMPLE: &str = "id provider account key observedAt receivedAt latitude longitude \
                      accuracyM altitudeM speedMps";
/// The time of the test hub, in Unix seconds: 2030-05-05T12:00:00Z.
const NOW: i64 = 1_904_212_800;

/// A hub with sealing keys in a folder of its own and the OwnTracks route beside `/graphql`.
struct LocationHub {
    app: Router,
    token: String,
    _keys: tempfile::TempDir,
}

impl LocationHub {
    async fn new(pool: &PgPool) -> Self {
        let keys = tempfile::tempdir().unwrap();
        let sealing = SealingKeys::open(Some(keys.path()), b"credential").unwrap();
        Self::with(pool, Store::in_folder(None, sealing), keys).await
    }

    async fn with(pool: &PgPool, files: Store, keys: tempfile::TempDir) -> Self {
        set_operator(pool).await;
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(start()));
        let parts = Dependencies {
            files,
            ..dependencies(pool, clock.clone(), Bus::new(64))
        };
        let routes = anima_hub::schema::engine_routes(&parts);
        let app = router(build_schema(parts), pool.clone(), clock).merge(routes);
        let token = Hub::new(pool, start()).token().await;
        Self {
            app,
            token,
            _keys: keys,
        }
    }

    async fn send(&self, mut request: Request<Body>) -> (StatusCode, Value) {
        let peer: SocketAddr = format!("{CLIENT}:50000").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn post(&self, query: &str, variables: Value) -> Value {
        let body = json!({ "query": query, "variables": variables }).to_string();
        let request = Request::post("/graphql")
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();
        self.send(request).await.1
    }

    /// Posts `body` as the OwnTracks app does, with `password`.
    async fn owntracks(&self, password: &str, body: &str) -> (StatusCode, Value) {
        let basic = STANDARD.encode(format!("phone:{password}"));
        let request = Request::post("/locations/v1/owntracks")
            .header(header::AUTHORIZATION, format!("Basic {basic}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        self.send(request).await
    }

    /// Makes an ingest token and returns `(id, value)`.
    async fn ingest_token(&self, name: &str) -> (String, String) {
        let made = self
            .post(
                "mutation($n: String!) { createLocationIngestToken(name: $n) \
                 { token ingestToken { id name lastUsedAt revokedAt } } }",
                json!({ "n": name }),
            )
            .await;
        let issued = &made["data"]["createLocationIngestToken"];
        assert_eq!(issued["ingestToken"]["name"], name, "{made}");
        (
            issued["ingestToken"]["id"].as_str().unwrap().to_owned(),
            issued["token"].as_str().unwrap().to_owned(),
        )
    }

    /// The samples of the day around the time of the hub.
    async fn samples(&self) -> Value {
        let query = format!(
            "query($f: DateTime!, $t: DateTime!) {{ locationSamples(from: $f, to: $t) \
             {{ {SAMPLE} }} }}"
        );
        let span = json!({ "f": "2030-05-04T12:00:00Z", "t": "2030-05-06T12:00:00Z" });
        self.post(&query, span).await
    }
}

fn fix(tst: i64) -> String {
    json!({ "_type": "location", "tst": tst, "lat": 52.0, "lon": 4.0, "acc": 12, "tid": "ph" })
        .to_string()
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("select count(*) from {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Every stored text of the event log and the audit log, in one string.
async fn logs(pool: &PgPool) -> String {
    sqlx::query_scalar(
        "select coalesce(string_agg(t, ' '), '') from ( \
         select message || ' ' as t from events.event_log union all \
         select change::text from events.audit_log) logs",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn r_145_samples_of_a_phone_app_are_safe_to_send_again_and_keep_a_missing_number(
    pool: PgPool,
) {
    let hub = LocationHub::new(&pool).await;
    let record = "mutation($s: [LocationSampleInput!]!) { recordLocationSamples(samples: $s) }";
    let batch = json!({ "s": [
        { "key": "a", "observedAt": "2030-05-05T12:00:00.123456Z", "latitude": 52.0,
          "longitude": 4.0 },
        { "key": "b" },
    ] });
    let first = hub.post(record, batch.clone()).await;
    assert_eq!(first["data"]["recordLocationSamples"], 2, "{first}");
    let again = hub.post(record, batch).await;
    assert_eq!(again["data"]["recordLocationSamples"], 0, "{again}");
    assert_eq!(count(&pool, "locations.sample").await, 2);

    let read = hub.samples().await;
    let samples = read["data"]["locationSamples"].as_array().unwrap();
    assert_eq!(
        samples.len(),
        1,
        "a sample with no time is in no span: {read}"
    );
    assert_eq!(samples[0]["provider"], "ANIMA");
    assert_eq!(samples[0]["observedAt"], "2030-05-05T12:00:00.123456+00:00");
    assert!(samples[0]["accuracyM"].is_null(), "no number in its place");
    let events: i64 = sqlx::query_scalar(
        "select count(*) from events.audit_log where target_type = 'location_samples' \
         and action = 'create'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 1, "the batch that stored nothing wrote no entry");

    let bad = hub
        .post(
            record,
            json!({ "s": [{ "key": "c", "latitude": 95.0, "longitude": 4.0 }] }),
        )
        .await;
    assert_eq!(error(&bad).1, "VALIDATION");
    assert_eq!(
        bad["errors"][0]["extensions"]["fields"][0]["field"],
        "samples[0].latitude"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn r_145_a_read_of_samples_checks_its_span_and_needs_the_sealing_keys(pool: PgPool) {
    let hub = LocationHub::new(&pool).await;
    let query = "query($f: DateTime!, $t: DateTime!, $n: Int) \
                 { locationSamples(from: $f, to: $t, first: $n) { id } }";
    for (from, to, first) in [
        ("2026-01-02T00:00:00Z", "2026-01-01T00:00:00Z", 10),
        ("2026-01-01T00:00:00Z", "2026-02-02T00:00:00Z", 10),
        ("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z", 1001),
    ] {
        let refused = hub
            .post(query, json!({ "f": from, "t": to, "n": first }))
            .await;
        assert_eq!(error(&refused).1, "VALIDATION", "{from} {to} {first}");
    }

    let keys = tempfile::tempdir().unwrap();
    let without = LocationHub::with(&pool, Store::off(), keys).await;
    let record = "mutation($s: [LocationSampleInput!]!) { recordLocationSamples(samples: $s) }";
    let refused = without.post(record, json!({ "s": [{ "key": "a" }] })).await;
    assert_eq!(error(&refused).1, "UNAVAILABLE");
}
