//! MET-012, K8 Section 3: a code is made by a signed-in operator, works once within 15
//! minutes, and makes earlier unused codes of its device useless; enrolling replaces the
//! device's token and sets its metrics source to `agent` (MET-011).

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{Agents, PERMITTED, PVE_01, metrics_events, refusal, source};
use crate::hub::{error, start};

fn time(value: &Value) -> DateTime<Utc> {
    value.as_str().unwrap().parse().unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_a_code_works_once_and_enrols_the_device(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let made = agents.enrol_agent_response(PVE_01).await;
    let enrolment = &made["data"]["enrolAgent"];
    let code = enrolment["code"].as_str().unwrap();
    assert!(
        code.starts_with("anima_enrol_") && code.len() == 12 + 22,
        "{code}"
    );
    assert_eq!(
        time(&enrolment["expiresAt"]),
        start() + TimeDelta::minutes(15)
    );

    let enrolled = agents.enrolled(code).await;
    assert_eq!(enrolled["device"], PVE_01);
    assert_eq!(enrolled["deviceName"], "pve-01");
    let token = enrolled["token"].as_str().unwrap();
    assert!(
        token.starts_with("anima_agent_") && token.len() == 12 + 43,
        "{token}"
    );
    assert_eq!(time(&enrolled["expiresAt"]), start() + TimeDelta::days(365));

    let again = agents.enrol(code).await;
    assert_eq!(
        again,
        (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"))
    );
    assert_eq!(source(&pool, PVE_01).await, "agent");
    let infos = metrics_events(&pool, "info").await;
    assert!(
        infos.contains(
            &"The agent on pve-01 enrolled; its metrics now come from the agent.".to_owned()
        ),
        "{infos:?}"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_a_code_stops_working_at_15_minutes(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let late = agents.code_for(PVE_01).await;
    agents.clock.advance(TimeDelta::minutes(15));
    assert_eq!(
        agents.enrol(&late).await,
        (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"))
    );

    let in_time = agents.code_for(PVE_01).await;
    agents
        .clock
        .advance(TimeDelta::minutes(15) - TimeDelta::seconds(1));
    assert_eq!(agents.enrol(&in_time).await.0, StatusCode::OK);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_a_new_code_makes_the_older_unused_one_useless(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let older = agents.code_for(PVE_01).await;
    let newer = agents.code_for(PVE_01).await;
    assert_eq!(
        agents.enrol(&older).await,
        (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"))
    );
    assert_eq!(agents.enrol(&newer).await.0, StatusCode::OK);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_enrolling_again_replaces_the_token(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let first = agents.token_for(PVE_01).await;
    let second = agents.token_for(PVE_01).await;
    assert_eq!(
        agents.renew(&first).await,
        (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"))
    );
    assert_eq!(agents.renew(&second).await.0, StatusCode::OK);
    assert_eq!(source(&pool, PVE_01).await, "agent");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k8_every_bad_enrol_body_is_the_same_refusal_and_a_large_one_is_too_large(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let code = agents.code_for(PVE_01).await;
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
        let answer = agents.enrol_body(body.clone(), PERMITTED).await;
        assert_eq!(
            answer,
            (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED")),
            "{body}"
        );
    }
    let padded = format!(r#"{{"code":"{code}","pad":"{}"}}"#, "x".repeat(4096));
    let too_large = agents.enrol_body(padded, PERMITTED).await;
    assert_eq!(
        too_large,
        (StatusCode::PAYLOAD_TOO_LARGE, refusal("TOO_LARGE"))
    );
    // None of the refusals used the code; unknown members are ignored.
    let extra = json!({ "code": code, "agentVersion": "0.1.0" }).to_string();
    let accepted = agents.enrol_body(extra, PERMITTED).await;
    assert_eq!(accepted.0, StatusCode::OK, "{}", accepted.1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_012_only_a_signed_in_operator_makes_a_code_for_a_device_that_exists(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let query = "mutation($id: ID!) { enrolAgent(deviceId: $id) { code } }";
    let signed_out = agents.graphql(None, query, json!({ "id": PVE_01 })).await;
    assert_eq!(error(&signed_out).1, "UNAUTHENTICATED");
    let unknown = "0190a000-0000-7000-8002-0000000000ff";
    let missing = agents.operate(query, json!({ "id": unknown })).await;
    assert_eq!(
        error(&missing),
        ("No such device.".to_owned(), "NOT_FOUND".to_owned())
    );
    let malformed = agents.operate(query, json!({ "id": "pve-01" })).await;
    assert_eq!(error(&malformed).1, "VALIDATION");
}
