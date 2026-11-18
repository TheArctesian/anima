//! MET-012, K8 Section 3: a code is made by a signed-in operator, works once within 15
//! minutes, and makes earlier unused codes of its device useless; enrolling replaces the
//! device's token and sets its metrics source to `daemon` (MET-011).

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, TimeDelta, Utc};
use daemon_core::wire::{DEVICE_NAME_CHARS, is_device_name};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    DOCKER_HOST, Daemons, PERMITTED, PVE_01, metrics_events, refusal, rows_of, source,
};
use crate::hub::{error, start};

fn time(value: &Value) -> DateTime<Utc> {
    value.as_str().unwrap().parse().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_a_code_works_once_and_enrols_the_device(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let made = daemons.enrol_daemon_response(PVE_01).await;
    let enrolment = &made["data"]["enrolDaemon"];
    let code = enrolment["code"].as_str().unwrap();
    assert!(
        code.starts_with("anima_enrol_") && code.len() == 12 + 22,
        "{code}"
    );
    assert_eq!(
        time(&enrolment["expiresAt"]),
        start() + TimeDelta::minutes(15)
    );

    let enrolled = daemons.enrolled(code).await;
    assert_eq!(enrolled["device"], PVE_01);
    assert_eq!(enrolled["deviceName"], "pve-01");
    let token = enrolled["token"].as_str().unwrap();
    assert!(
        token.starts_with("anima_daemon_") && token.len() == 13 + 43,
        "{token}"
    );
    assert_eq!(time(&enrolled["expiresAt"]), start() + TimeDelta::days(365));

    let again = daemons.enrol(code).await;
    assert_eq!(
        again,
        (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"))
    );
    assert_eq!(source(&pool, PVE_01).await, "daemon");
    let infos = metrics_events(&pool, "info").await;
    assert!(
        infos.contains(
            &"The daemon on pve-01 enrolled; its metrics now come from the daemon.".to_owned()
        ),
        "{infos:?}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_a_code_stops_working_at_15_minutes(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let late = daemons.code_for(PVE_01).await;
    daemons.clock.advance(TimeDelta::minutes(15));
    assert_eq!(
        daemons.enrol(&late).await,
        (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"))
    );

    let in_time = daemons.code_for(PVE_01).await;
    daemons
        .clock
        .advance(TimeDelta::minutes(15) - TimeDelta::seconds(1));
    assert_eq!(daemons.enrol(&in_time).await.0, StatusCode::OK);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_a_new_code_makes_the_older_unused_one_useless(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let older = daemons.code_for(PVE_01).await;
    let newer = daemons.code_for(PVE_01).await;
    assert_eq!(
        daemons.enrol(&older).await,
        (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"))
    );
    assert_eq!(daemons.enrol(&newer).await.0, StatusCode::OK);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_uses_of_one_code_at_once_enrol_once(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let code = daemons.code_for(PVE_01).await;
    let uses = (0..8).map(|_| daemons.enrol(&code));
    let answers = futures_util::future::join_all(uses).await;
    let accepted = answers
        .iter()
        .filter(|(status, _)| *status == StatusCode::OK);
    assert_eq!(accepted.count(), 1, "{answers:?}");
    // The rest are refused, or sent back while the hub answers as many daemon requests as it
    // takes at once (MET-017); those never reached the code.
    let refused = (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"));
    let unavailable = (StatusCode::SERVICE_UNAVAILABLE, refusal("UNAVAILABLE"));
    let others = answers
        .iter()
        .filter(|(status, _)| *status != StatusCode::OK);
    assert!(
        others
            .clone()
            .all(|answer| *answer == refused || *answer == unavailable),
        "{answers:?}"
    );
    assert_eq!(others.count(), 7);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_a_new_code_leaves_other_devices_codes_working(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let pve = daemons.code_for(PVE_01).await;
    let docker = daemons.code_for(DOCKER_HOST).await;
    let enrolled = daemons.enrolled(&pve).await;
    assert_eq!(enrolled["device"], PVE_01);
    assert_eq!(rows_of(&pool, "daemons", DOCKER_HOST).await, 0);
    let enrolled = daemons.enrolled(&docker).await;
    assert_eq!(enrolled["device"], DOCKER_HOST);
    assert_eq!(rows_of(&pool, "daemons", DOCKER_HOST).await, 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_enrolling_again_replaces_the_token(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let first = daemons.token_for(PVE_01).await;
    let second = daemons.token_for(PVE_01).await;
    assert_eq!(
        daemons.renew(&first).await,
        (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"))
    );
    assert_eq!(daemons.renew(&second).await.0, StatusCode::OK);
    assert_eq!(source(&pool, PVE_01).await, "daemon");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k8_every_bad_enrol_body_is_the_same_refusal_and_a_large_one_is_too_large(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let token = daemons.token_for(PVE_01).await;
    let code = daemons.code_for(PVE_01).await;
    let bodies = [
        String::new(),
        "not json".to_owned(),
        "{}".to_owned(),
        "[]".to_owned(),
        json!({ "code": 5 }).to_string(),
        json!({ "code": "anima_enrol_short" }).to_string(),
        json!({ "code": format!("{code}=") }).to_string(),
        json!({ "code": code.to_uppercase() }).to_string(),
        json!({ "code": token }).to_string(),
        json!({ "token": code }).to_string(),
    ];
    for body in bodies {
        let answer = daemons.enrol_body(body.clone(), PERMITTED).await;
        assert_eq!(
            answer,
            (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED")),
            "{body}"
        );
    }
    let padded = format!(r#"{{"code":"{code}","pad":"{}"}}"#, "x".repeat(4096));
    let too_large = daemons.enrol_body(padded, PERMITTED).await;
    assert_eq!(
        too_large,
        (StatusCode::PAYLOAD_TOO_LARGE, refusal("TOO_LARGE"))
    );
    // None of the refusals used the code; unknown members are ignored.
    let extra = json!({ "code": code, "daemonVersion": "0.1.0" }).to_string();
    let accepted = daemons.enrol_body(extra, PERMITTED).await;
    assert_eq!(accepted.0, StatusCode::OK, "{}", accepted.1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k8_a_declared_length_past_4_kib_is_too_large_before_the_body_is_read(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let code = daemons.code_for(PVE_01).await;
    let body = json!({ "code": code }).to_string();
    let request = Request::post("/daemon/v1/enrol")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, "4097")
        .body(Body::from(body))
        .unwrap();
    let answer = daemons.send(request, PERMITTED).await;
    assert_eq!(
        answer,
        (StatusCode::PAYLOAD_TOO_LARGE, refusal("TOO_LARGE"))
    );
    // The code was never read, so it still works.
    assert_eq!(daemons.enrol(&code).await.0, StatusCode::OK);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_only_a_signed_in_operator_makes_a_code_for_a_device_that_exists(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let query = "mutation($id: ID!) { enrolDaemon(deviceId: $id) { code } }";
    let signed_out = daemons.graphql(None, query, json!({ "id": PVE_01 })).await;
    assert_eq!(error(&signed_out).1, "UNAUTHENTICATED");
    let unknown = "0190a000-0000-7000-8002-0000000000ff";
    let missing = daemons.operate(query, json!({ "id": unknown })).await;
    assert_eq!(
        error(&missing),
        ("No such device.".to_owned(), "NOT_FOUND".to_owned())
    );
    let malformed = daemons.operate(query, json!({ "id": "pve-01" })).await;
    assert_eq!(error(&malformed).1, "VALIDATION");
}

/// T8 S1: the longest name the device rule takes (`DEVICE_NAME_CHARS` characters, 192 bytes
/// here) reaches the daemon as a name it keeps, so no code is spent on an answer the daemon must
/// throw away.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_every_device_name_the_hub_takes_is_one_the_daemon_keeps(pool: PgPool) {
    let daemons = Daemons::new(&pool).await;
    let rename = "mutation($id: ID!, $input: UpdateDeviceInput!) { \
                  updateDevice(id: $id, input: $input) { name } }";
    let longest = "漢".repeat(DEVICE_NAME_CHARS);
    let longer = json!({ "id": PVE_01, "input": { "name": format!("{longest}漢") } });
    let refused = daemons.operate(rename, longer).await;
    assert!(refused["data"].is_null(), "{refused}");
    let renamed = daemons
        .operate(
            rename,
            json!({ "id": PVE_01, "input": { "name": longest } }),
        )
        .await;
    assert_eq!(
        renamed["data"]["updateDevice"]["name"], longest,
        "{renamed}"
    );

    let code = daemons.code_for(PVE_01).await;
    let enrolled = daemons.enrolled(&code).await;
    let name = enrolled["deviceName"].as_str().unwrap();
    assert_eq!(name, longest);
    assert!(is_device_name(name), "{} bytes", name.len());
}
