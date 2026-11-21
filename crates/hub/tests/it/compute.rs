//! The device policy of a compute node through the hub's routes (CMP-002, CAP-010 to CAP-012;
//! compute plan task L1-5): `devicePolicies` reads the switch "takes work" of the work route and
//! the last hardware values; `setDevicePolicy` changes the policy and never the switch.

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::daemon::{
    batches::cpu_batch,
    harness::{DOCKER_HOST, Daemons, PERMITTED, PVE_01, pushing_now},
};

const POLICIES: &str = "{ devicePolicies { device { id } memorySharePercent diskSharePercent
    quietHours { start end timeZone } mainsOnly idleOnly isDefault takesModelWork stopReason } }";

const SET: &str = "mutation($deviceId: ID!, $input: DevicePolicyInput!) {
    setDevicePolicy(deviceId: $deviceId, input: $input) { takesModelWork stopReason isDefault } }";

/// A claim of `device` on the work route with `switch`; the hub stores the switch.
async fn claim(daemons: &Daemons, token: &str, device: &str, switch: &str) {
    let body = json!({
        "device": device, "switch": switch, "types": ["build"], "kinds": ["container"],
        "os": "linux", "architecture": "x86_64",
        "free": { "slots": 1, "memoryBytes": 8_589_934_592_u64, "diskBytes": 68_719_476_736_u64 },
        "holds": [], "unclean": false
    });
    let request = Request::post("/work/v1/claim")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap();
    let (status, text) = daemons.send(request, PERMITTED).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{text}");
}

async fn policy_of(daemons: &Daemons, device: &str) -> Value {
    let answer = daemons.operate(POLICIES, json!({})).await;
    answer["data"]["devicePolicies"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"))
        .iter()
        .find(|row| row["device"]["id"] == device)
        .cloned()
        .unwrap_or_else(|| panic!("{device} in {answer}"))
}

async fn set(daemons: &Daemons, id: &str, input: Value) -> Value {
    let variables = json!({ "deviceId": id, "input": input });
    daemons.operate(SET, variables).await["data"]["setDevicePolicy"].clone()
}

fn input(mains_only: bool, idle_only: bool) -> Value {
    json!({ "memorySharePercent": 60, "diskSharePercent": 40, "quietHours": null,
            "mainsOnly": mains_only, "idleOnly": idle_only })
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn cmp_002_a_machine_whose_switch_is_off_gets_no_model_work(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let off = daemons.token_for(PVE_01).await;
    let on = daemons.token_for(DOCKER_HOST).await;

    claim(&daemons, &off, PVE_01, "off").await;
    claim(&daemons, &on, DOCKER_HOST, "on").await;

    let switched_off = policy_of(&daemons, PVE_01).await;
    let switched_on = policy_of(&daemons, DOCKER_HOST).await;
    assert_eq!(switched_off["takesModelWork"], false);
    assert_eq!(switched_off["stopReason"], "SWITCH_OFF");
    assert_eq!(switched_on["takesModelWork"], true, "{switched_on}");
    assert_eq!(
        switched_on,
        json!({ "device": { "id": DOCKER_HOST }, "memorySharePercent": 100,
                "diskSharePercent": 75, "quietHours": null, "mainsOnly": true,
                "idleOnly": false, "isDefault": true, "takesModelWork": true, "stopReason": null })
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn cap_011_cap_012_battery_and_an_active_operator_stop_model_work_as_the_policy_says(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let token = daemons.token_for(PVE_01).await;
    let mut pushed = cpu_batch(PVE_01, now - TimeDelta::seconds(10), 5.0);
    pushed["hardware"] = json!({ "systemMemoryBytes": 17_179_869_184_u64, "cpuFeatures": [],
        "backends": [], "powerSource": "battery", "operatorActive": true });
    assert_eq!(
        daemons.push(&token, &pushed).await.0,
        StatusCode::NO_CONTENT
    );
    claim(&daemons, &token, PVE_01, "on").await;
    let id = PVE_01;

    let by_default = policy_of(&daemons, PVE_01).await;
    let idle_only = set(&daemons, id, input(false, true)).await;
    let neither = set(&daemons, id, input(false, false)).await;

    assert_eq!(by_default["stopReason"], "ON_BATTERY");
    assert_eq!(idle_only["stopReason"], "OPERATOR_ACTIVE");
    assert_eq!(
        neither,
        json!({ "takesModelWork": true, "stopReason": null, "isDefault": false })
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn cap_010_quiet_hours_are_stored_and_a_value_out_of_range_is_refused(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let id = PVE_01;
    let mut wanted = input(true, false);
    wanted["quietHours"] = json!({ "start": "22:00", "end": "07:00", "timeZone": "Europe/London" });
    let mut wrong = input(true, false);
    wrong["memorySharePercent"] = json!(0);

    let refused = daemons
        .operate(SET, json!({ "deviceId": id, "input": wrong }))
        .await;
    daemons
        .operate(SET, json!({ "deviceId": id, "input": wanted }))
        .await;

    assert_eq!(
        refused["errors"][0]["extensions"]["code"], "VALIDATION",
        "{refused}"
    );
    let stored = policy_of(&daemons, PVE_01).await;
    assert_eq!(
        stored["quietHours"],
        json!({ "start": "22:00", "end": "07:00", "timeZone": "Europe/London" })
    );
    assert_eq!(
        stored["stopReason"], "NO_SWITCH",
        "the machine never asked for work"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../fixtures", scripts("devices"))
)]
async fn r_new_devicepolicy_an_api_key_cannot_set_a_device_policy(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let daemons = Daemons::at(&pool, now).await;
    let made = daemons
        .operate(
            "mutation($input: ApiKeyInput!) { createApiKey(input: $input) { secret } }",
            json!({ "input": { "name": "writer", "scope": "WRITE", "madeFor": "AGENT",
                               "expiresAt": now + TimeDelta::days(30) } }),
        )
        .await;
    let key = made["data"]["createApiKey"]["secret"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned();
    let variables = json!({ "deviceId": PVE_01, "input": input(false, true) });

    let refused = daemons.graphql(Some(&key), SET, variables).await;
    let read = daemons.graphql(Some(&key), POLICIES, json!({})).await;

    assert_eq!(
        refused["errors"][0]["extensions"]["code"], "FORBIDDEN",
        "{refused}"
    );
    assert!(read["data"]["devicePolicies"].is_array(), "{read}");
    assert_eq!(policy_of(&daemons, PVE_01).await["isDefault"], true);
}
