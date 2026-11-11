//! A heartbeat queues its notifications in its own transaction, through the push route as the
//! hub mounts it: one row per selected channel, none under maintenance (an Up after an
//! announced Down included), and a pushed message and name cut to their caps (NOT-001,
//! NOT-007, D-16, Section 6.5, LOG-007).

use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use anima_hub::{
    checks::push_router,
    clock::{Clock, FixedClock},
    events::Bus,
    schema::{HubSchema, build_schema},
};
use async_graphql::{Response, Variables};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::{
    hub::dependencies,
    notifications::harness::{create_query, signed_in, webhook},
};

pub(super) const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
/// `jellyfin` in the K6 fixtures: its manual maintenance flag is set.
const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";

/// A clock one second later on every call, so each push is a newer beat.
struct Stepping(AtomicI64, DateTime<Utc>);

impl Clock for Stepping {
    fn now(&self) -> DateTime<Utc> {
        self.1 + TimeDelta::seconds(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

/// The hub's schema and its push route, both at the database clock.
pub(super) struct Rig {
    pub(super) pool: PgPool,
    pub(super) schema: HubSchema,
    router: Router,
}

pub(super) async fn rig(pool: &PgPool) -> Rig {
    let at: DateTime<Utc> = sqlx::query_scalar("SELECT pg_catalog.now()")
        .fetch_one(pool)
        .await
        .unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(at));
    let schema = build_schema(dependencies(pool, clock, Bus::new(16)));
    let stepping: Arc<dyn Clock> = Arc::new(Stepping(AtomicI64::new(0), at));
    Rig {
        pool: pool.clone(),
        schema,
        router: push_router(pool.clone(), Bus::new(16), stepping),
    }
}

impl Rig {
    /// The whole response to a signed-in request.
    pub(super) async fn run(&self, query: &str, variables: Value) -> Response {
        let request = signed_in(query).variables(Variables::from_json(variables));
        self.schema.execute(request).await
    }

    /// The data of a signed-in request that must have no errors.
    pub(super) async fn data(&self, query: &str, variables: Value) -> Value {
        let response = self.run(query, variables).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        response.data.into_json().unwrap()
    }

    /// A new webhook channel; returns its id.
    pub(super) async fn channel(&self, name: &str, on_by_default: bool) -> String {
        let mut input = webhook(name);
        input["onByDefault"] = json!(on_by_default);
        let created = self.data(&create_query(), json!({ "input": input })).await;
        created["createNotificationChannel"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// A new check on `device`; returns its id.
    pub(super) async fn check(&self, device: &str, name: &str, config: Value) -> String {
        let query = "mutation($input: CreateCheckInput!) { createCheck(input: $input) { id } }";
        let input = json!({ "deviceId": device, "name": name, "retries": 0, "config": config });
        let created = self.data(query, json!({ "input": input })).await;
        created["createCheck"]["id"].as_str().unwrap().to_owned()
    }

    /// A push check on `device` that notifies `channels`; returns its id and push path.
    pub(super) async fn push_check(
        &self,
        device: &str,
        name: &str,
        channels: &[&str],
    ) -> (String, String) {
        let id = self.check(device, name, json!({ "push": true })).await;
        let set = "mutation($check: ID!, $input: CheckNotificationsInput!) { \
            setCheckNotifications(checkId: $check, input: $input) { resendEveryBeats } }";
        let input = json!({ "channelIds": channels });
        self.data(set, json!({ "check": id, "input": input })).await;
        let query =
            "query($id: ID!) { check(id: $id) { config { ... on PushConfig { pushPath } } } }";
        let read = self.data(query, json!({ "id": id })).await;
        let path = read["check"]["config"]["pushPath"]
            .as_str()
            .unwrap()
            .to_owned();
        (id, path)
    }

    /// Pushes `query` to `path`; the route must answer 200.
    pub(super) async fn push(&self, path: &str, query: &str) {
        let request = Request::get(format!("{path}?{query}"))
            .body(Body::empty())
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// The `(kind, payload)` of each row queued for `check`, oldest first.
    pub(super) async fn queued(&self, check: &str) -> Vec<(String, Value)> {
        sqlx::query_as(
            "SELECT kind, payload FROM notifications.outbox WHERE check_id = $1::text::uuid \
             ORDER BY created_at, kind, channel_id",
        )
        .bind(check)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    /// The stored heartbeat statuses of `check`, oldest first.
    pub(super) async fn beats(&self, check: &str) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT status FROM checks.heartbeats WHERE check_id = $1::text::uuid ORDER BY at",
        )
        .bind(check)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }
}

pub(super) fn kinds(rows: &[(String, Value)]) -> Vec<&str> {
    rows.iter().map(|(kind, _)| kind.as_str()).collect()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn not_001_a_pushed_down_queues_one_row_per_selected_channel_and_up_follows(pool: PgPool) {
    let rig = rig(&pool).await;
    let a = rig.channel("Alpha", false).await;
    let b = rig.channel("Bravo", false).await;
    let _unselected = rig.channel("Charlie", false).await;
    let (id, path) = rig.push_check(PVE_01, "Backup job", &[&a, &b]).await;

    rig.push(&path, "status=down&msg=disk%20full").await;
    rig.push(&path, "status=down").await;
    rig.push(&path, "status=up").await;

    let rows = rig.queued(&id).await;
    assert_eq!(kinds(&rows), ["down", "down", "up", "up"]);
    let (_, payload) = &rows[0];
    assert_eq!(payload["device"], "pve-01");
    assert_eq!(payload["check"], "Backup job");
    assert_eq!(payload["status"], "down");
    assert_eq!(payload["message"], "disk full");
    assert_eq!(payload["previous"], "unknown", "a new check");
    let (_, up) = &rows[2];
    assert_eq!(up["status"], "up");
    assert_eq!(up["previous"], "down");
    assert_eq!(rig.beats(&id).await, ["down", "down", "up"]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn not_001_a_pushed_down_under_maintenance_queues_nothing(pool: PgPool) {
    let rig = rig(&pool).await;
    let a = rig.channel("Alpha", false).await;
    let (id, path) = rig.push_check(JELLYFIN, "Backup job", &[&a]).await;

    rig.push(&path, "status=down").await;

    assert_eq!(rig.beats(&id).await, ["maintenance"]);
    assert_eq!(rig.queued(&id).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn not_001_an_up_pushed_once_maintenance_starts_queues_nothing(pool: PgPool) {
    let rig = rig(&pool).await;
    let a = rig.channel("Alpha", false).await;
    let (id, path) = rig.push_check(PVE_01, "Backup job", &[&a]).await;
    rig.push(&path, "status=down").await;
    sqlx::query("UPDATE devices.devices SET maintenance = true WHERE id = $1::text::uuid")
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap();

    rig.push(&path, "status=up").await;

    assert_eq!(rig.beats(&id).await, ["down", "up"]);
    assert_eq!(
        kinds(&rig.queued(&id).await),
        ["down"],
        "no Up under maintenance"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn not_007_a_pushed_message_is_cut_at_500_characters_and_a_200_character_name_is_kept(
    pool: PgPool,
) {
    let rig = rig(&pool).await;
    let a = rig.channel("Alpha", false).await;
    let name = "n".repeat(200);
    let (id, path) = rig.push_check(PVE_01, &name, &[&a]).await;

    rig.push(&path, &format!("status=down&msg={}", "m".repeat(1000)))
        .await;

    let rows = rig.queued(&id).await;
    assert_eq!(rows.len(), 1);
    let payload = &rows[0].1;
    assert_eq!(payload["message"], "m".repeat(500));
    assert_eq!(payload["check"], name);
}
