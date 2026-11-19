//! The container queries through a schema that holds them alone (HOST-001, API-003,
//! K7): the sign-in, the values the hub refuses before it reaches a device, and a device the
//! hub cannot read. The reads against an SSH server are in the crate (`work/containers/tests`).

use std::sync::Arc;

use anima_hub::{
    clock::{Clock, FixedClock},
    fleet::{
        access::Identity,
        events::{Actor, Operator},
        targets::AddressPolicy,
    },
    ids::OperatorId,
    work::containers::graphql::Query,
};
use async_graphql::{EmptyMutation, EmptySubscription, Request, Response, Schema, Value};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
/// `docker-host` of the devices fixture: SSH on, no accepted host key, no stored credential.
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const ID: &str = "4f5c1d2e3a4b5c6d7e8f90123456789abcdef0123456789abcdef0123456789a";

fn now() -> DateTime<Utc> {
    "2026-11-18T09:00:00Z".parse().unwrap()
}

fn schema(pool: PgPool) -> Schema<Query, EmptyMutation, EmptySubscription> {
    Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(pool)
        .data(crate::hub::test_vault())
        .data(AddressPolicy::default())
        .data(Arc::new(FixedClock(now())) as Arc<dyn Clock>)
        .finish()
}

fn signed_in(query: &str) -> Request {
    let operator = Operator {
        id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
        actor: Actor::for_tests(),
    };
    Identity::for_tests(operator, now() + TimeDelta::days(1)).attach(Request::new(query))
}

fn list_of(device: &str) -> String {
    format!(r#"{{ containers(deviceId: "{device}") {{ name state }} }}"#)
}

fn log_of(container: &str, lines: &str) -> String {
    format!(
        r#"{{ containerLog(deviceId: "{DOCKER_HOST}", containerId: "{container}"{lines}) {{ lines {{ at text }} cut }} }}"#
    )
}

fn detail_of(container: &str) -> String {
    format!(
        r#"{{ containerDetail(deviceId: "{DOCKER_HOST}", containerId: "{container}") {{ name environmentNames }} }}"#
    )
}

fn storage_of(device: &str) -> String {
    format!(
        r#"{{ containerStorage(deviceId: "{device}") {{ images {{ id }} usage {{ type }} }} }}"#
    )
}

fn networks_of(device: &str) -> String {
    format!(r#"{{ containerNetworks(deviceId: "{device}") {{ name containers }} }}"#)
}

/// The one error's code and message.
fn error_of(response: &Response) -> (String, String) {
    assert_eq!(response.data, Value::Null);
    let [error] = response.errors.as_slice() else {
        panic!("expected one error, got {:?}", response.errors);
    };
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    let Some(Value::String(code)) = code else {
        panic!("no code: {error:?}");
    };
    (code.clone(), error.message.clone())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn api_003_each_query_refuses_a_caller_who_is_not_signed_in(pool: PgPool) {
    let schema = schema(pool);
    for document in [
        list_of(DOCKER_HOST),
        log_of(ID, ""),
        detail_of(ID),
        storage_of(DOCKER_HOST),
        networks_of(DOCKER_HOST),
    ] {
        let (code, _) = error_of(&schema.execute(document).await);
        assert_eq!(code, "UNAUTHENTICATED");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn host_001_a_value_the_hub_does_not_accept_is_validation(pool: PgPool) {
    let schema = schema(pool);
    for document in [
        log_of("dawarich_app; reboot", ""),
        log_of("$(reboot)aaaaaaa", ""),
        log_of(&ID[..11], ""),
        log_of(&ID.to_uppercase(), ""),
        log_of(ID, ", lines: 0"),
        log_of(ID, ", lines: 1001"),
        detail_of("dawarich_app; reboot"),
        detail_of(&ID[..11]),
        list_of("not-an-id"),
        storage_of("not-an-id"),
        networks_of("not-an-id"),
    ] {
        let (code, _) = error_of(&schema.execute(signed_in(&document)).await);
        assert_eq!(code, "VALIDATION", "{document}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn host_001_an_unknown_device_is_not_found(pool: PgPool) {
    let schema = schema(pool);
    let other = list_of("0190a000-0000-7000-8002-0000000000ff");
    let (code, message) = error_of(&schema.execute(signed_in(&other)).await);
    assert_eq!(
        (code.as_str(), message.as_str()),
        ("NOT_FOUND", "No device has this ID.")
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn host_001_a_device_the_hub_holds_no_host_key_for_is_conflict_with_what_to_do(pool: PgPool) {
    let schema = schema(pool);
    for document in [
        list_of(DOCKER_HOST),
        log_of(ID, ", lines: 50"),
        detail_of(ID),
        storage_of(DOCKER_HOST),
        networks_of(DOCKER_HOST),
    ] {
        let (code, message) = error_of(&schema.execute(signed_in(&document)).await);
        assert_eq!(code, "CONFLICT", "{document}");
        assert_eq!(
            message,
            "Docker was not read: no host key is accepted for the device. Test its connection \
             and accept its key."
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn d_23_an_api_key_reads_the_list_and_never_a_log_or_the_details(pool: PgPool) {
    let schema = schema(pool);
    let with_key = |document: &str| {
        let operator = Operator {
            id: OPERATOR_ID.parse::<OperatorId>().unwrap(),
            name: "operator".to_owned(),
            actor: Actor::for_tests(),
        };
        Identity::api_key_for_tests(operator, now() + TimeDelta::days(1))
            .attach(Request::new(document))
    };
    for closed in [log_of(ID, ""), detail_of(ID)] {
        let (code, _) = error_of(&schema.execute(with_key(&closed)).await);
        assert_eq!(code, "FORBIDDEN", "{closed}");
    }
    // The list is open to a key: it gets as far as the device, which has no accepted key here.
    for open in [
        list_of(DOCKER_HOST),
        storage_of(DOCKER_HOST),
        networks_of(DOCKER_HOST),
    ] {
        let (code, _) = error_of(&schema.execute(with_key(&open)).await);
        assert_eq!(code, "CONFLICT", "{open}");
    }
}
