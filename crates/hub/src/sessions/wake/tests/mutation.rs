//! `wakeDevice` (K1, K7, LOG-005, LOG-013, API-003): the result, the audit entry `device.wake`
//! for each attempt on a known device, sent or not, the actor kind of the request on the entry
//! and the event, and each error code with its fixed message.

use std::sync::Arc;

use async_graphql::{EmptySubscription, Object, Request, Response, Schema, Value};
use chrono::TimeDelta;
use serde_json::{Value as Json, json};
use sqlx::PgPool;

use super::{Answer, MAC, PVE, assert_quiet, execs, exiting, relay, set_mac};
use crate::{
    access::Identity,
    clock::{Clock, FixedClock},
    events::Bus,
    sessions::{
        WakeMutation,
        tests::fixture::{DOCKER_HOST, address, now, operator},
    },
    ssh::tests::{Bench, reaching},
};

/// The address the operator's client calls from.
const CLIENT: &str = "100.64.0.7";

/// A root `Query`, which a schema needs; the test asks nothing of it.
struct Query;

#[Object]
impl Query {
    async fn up(&self) -> bool {
        true
    }
}

/// A schema with only the wake part, wired as `schema.rs` wires it.
fn schema(bench: &Bench) -> Schema<Query, WakeMutation, EmptySubscription> {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    Schema::build(Query, WakeMutation, EmptySubscription)
        .data(bench.db.clone())
        .data(bench.vault.clone())
        .data(reaching())
        .data(Bus::new(16))
        .data(clock)
        .finish()
}

fn mutation(device_id: &str) -> Request {
    Request::new(format!(
        r#"mutation {{ wakeDevice(deviceId: "{device_id}") {{ sentAt relayName }} }}"#
    ))
}

/// `wakeDevice` for `device_id`, signed in from [`CLIENT`].
async fn asked(bench: &Bench, device_id: &str) -> Response {
    let identity = Identity::for_tests(operator(), now() + TimeDelta::days(1));
    let request = identity.attach(mutation(device_id)).data(address(CLIENT));
    schema(bench).execute(request).await
}

/// The one error's code and message; its whole text is quiet.
fn error_of(bench: &Bench, response: &Response) -> (String, String) {
    assert_eq!(response.data, Value::Null);
    assert_eq!(response.errors.len(), 1, "{:?}", response.errors);
    let error = &response.errors[0];
    assert_quiet(&format!("{error:?}"), &bench.server);
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    let Some(Value::String(code)) = code else {
        panic!("no code: {error:?}");
    };
    (code.clone(), error.message.clone())
}

/// `(operator, action, target type, target id, client, change)` of each audit entry.
async fn audits(pool: &PgPool) -> Vec<(String, String, String, String, String, Json)> {
    sqlx::query_as(
        "select operator_name, action, target_type, target_id, host(client_addr), change \
         from events.audit_log",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The `device.wake` entry of a failed attempt on PVE with the error's `message`.
fn failed_entry(message: &str) -> (String, String, String, String, String, Json) {
    (
        "operator".to_owned(),
        "wake".to_owned(),
        "device".to_owned(),
        PVE.to_owned(),
        CLIENT.to_owned(),
        json!({ "sent": false, "reason": message }),
    )
}

/// The `actor_kind` of every row of `events.<table>`.
async fn kinds(pool: &PgPool, table: &str) -> Vec<Option<String>> {
    sqlx::query_scalar(&format!("select actor_kind from events.{table}"))
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_wake_device_returns_the_relay_and_writes_device_wake(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let response = asked(&bench, PVE).await;
    assert_eq!(response.errors, []);
    let data = response.data.into_json().unwrap();
    assert_eq!(
        data,
        json!({ "wakeDevice": { "sentAt": now().to_rfc3339(), "relayName": "docker-host" } })
    );
    let entry = (
        "operator".to_owned(),
        "wake".to_owned(),
        "device".to_owned(),
        PVE.to_owned(),
        CLIENT.to_owned(),
        json!({ "relayId": DOCKER_HOST, "relayName": "docker-host" }),
    );
    assert_eq!(audits(&pool).await, [entry]);
    assert_eq!(execs(&bench.server), [format!("wakeonlan {MAC}")]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k7_a_refusal_the_operator_can_fix_is_conflict_and_audited(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    set_mac(&pool, PVE, None).await;
    let (code, message) = error_of(&bench, &asked(&bench, PVE).await);
    assert_eq!(code, "CONFLICT");
    assert_eq!(
        message,
        "No Wake-on-LAN packet was sent: the device has no MAC address. Add it in the device's \
         remote access settings."
    );
    assert_eq!(audits(&pool).await, [failed_entry(&message)]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k7_a_failed_wakeonlan_is_unavailable_and_audited(pool: PgPool) {
    let bench = relay(&pool, exiting(2)).await;
    let (code, message) = error_of(&bench, &asked(&bench, PVE).await);
    assert_eq!(code, "UNAVAILABLE");
    assert_eq!(audits(&pool).await, [failed_entry(&message)]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn log_013_an_agent_s_wake_is_agent_on_the_entry_and_the_event_sent_or_not(pool: PgPool) {
    let bench = relay(&pool, exiting(2)).await;
    let identity = Identity::agent_key_for_tests(operator(), now() + TimeDelta::days(1));
    let request = identity.attach(mutation(PVE)).data(address(CLIENT));
    let (code, _) = error_of(&bench, &schema(&bench).execute(request).await);
    assert_eq!(code, "UNAVAILABLE");
    for table in ["audit_log", "event_log"] {
        assert_eq!(
            kinds(&pool, table).await,
            [Some("agent".to_owned())],
            "{table}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn k7_an_unknown_device_is_not_found_and_a_bad_id_is_validation(pool: PgPool) {
    let bench = relay(&pool, Answer::Echo).await;
    let missing = "0190a000-0000-7000-8002-0000000000ff";
    let (code, message) = error_of(&bench, &asked(&bench, missing).await);
    assert_eq!(code, "NOT_FOUND");
    assert_eq!(message, format!("No such device {missing}."));
    let (code, _) = error_of(&bench, &asked(&bench, "not-an-id").await);
    assert_eq!(code, "VALIDATION");
    assert_eq!(bench.server.connections(), 0);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn api_003_signed_out_wakes_nothing(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let response = schema(&bench).execute(mutation(PVE)).await;
    let (code, _) = error_of(&bench, &response);
    assert_eq!(code, "UNAUTHENTICATED");
    assert_eq!(bench.server.connections(), 0);
    assert_eq!(audits(&pool).await, []);
}
