//! K8 Sections 3 and 6, CR-15, CR-21, NET-002: every enrol refusal and every token refusal on
//! renew reaches the event log as a warning with its code and address, never the code or token;
//! one with no valid credential at most once a minute per address, one with a valid credential
//! at most once a minute per device. An address outside the permitted networks reaches neither
//! route.

use axum::http::StatusCode;
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{
    batches::cpu_batch,
    harness::{
        Agents, DOCKER_HOST, NEIGHBOUR, OUTSIDE, PERMITTED, PVE_01, metrics_events, pushing_now,
        refusal,
    },
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cr_15_refusals_without_a_credential_are_written_once_a_minute_per_address(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let guess = json!({ "code": format!("anima_enrol_{}", "A".repeat(22)) }).to_string();
    let junk = "Bearer anima_agent_nothing";
    for _ in 0..3 {
        agents.enrol_body(guess.clone(), PERMITTED).await;
        agents.renew_raw(Some(junk), "{}", PERMITTED).await;
    }
    agents.enrol_body("{}", NEIGHBOUR).await;
    assert_eq!(
        metrics_events(&pool, "warning").await,
        [
            "Refused an agent enrolment from 100.64.0.7: ENROLMENT_REFUSED.",
            "Refused an agent enrolment from 100.64.0.8: ENROLMENT_REFUSED.",
        ]
    );

    agents.clock.advance(TimeDelta::minutes(1));
    agents.renew_raw(Some(junk), "{}", PERMITTED).await;
    let written = metrics_events(&pool, "warning").await;
    assert_eq!(
        written.last().unwrap(),
        "Refused an agent token renewal from 100.64.0.7: TOKEN_REFUSED."
    );
    assert_eq!(written.len(), 3);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cr_21_refusals_with_a_valid_token_are_written_once_a_minute_per_device(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let bearer = format!("Bearer {token}");
    // From any address and on either route: the device decides, and every answer stays.
    for from in [PERMITTED, NEIGHBOUR, PERMITTED] {
        let answer = agents.renew_raw(Some(&bearer), "[]", from).await;
        assert_eq!(answer, (StatusCode::BAD_REQUEST, refusal("MALFORMED")));
        let answer = agents.push_raw(Some(&bearer), "[]", from).await;
        assert_eq!(answer, (StatusCode::BAD_REQUEST, refusal("MALFORMED")));
    }
    let first = (
        Some("pve-01".to_owned()),
        "Refused an agent token renewal from 100.64.0.7: MALFORMED.".to_owned(),
    );
    assert_eq!(device_warnings(&pool).await, std::slice::from_ref(&first));

    agents.clock.advance(TimeDelta::minutes(1));
    agents.push_raw(Some(&bearer), "[]", NEIGHBOUR).await;
    agents.push_raw(Some(&bearer), "[]", NEIGHBOUR).await;
    let next = (
        Some("pve-01".to_owned()),
        "Refused an agent push from 100.64.0.8: MALFORMED.".to_owned(),
    );
    assert_eq!(device_warnings(&pool).await, [first, next]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cr_18_cr_21_a_token_of_a_device_switched_away_cannot_flood_the_event_log(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let switch =
        "mutation($id: ID!) { updateDevice(id: $id, input: { metricsSource: SSH }) { id } }";
    agents.operate(switch, json!({ "id": PVE_01 })).await;
    for _ in 0..100 {
        let answer = agents.push(&token, &cpu_batch(PVE_01, now, 5.0)).await;
        assert_eq!(answer, (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED")));
    }
    assert_eq!(device_warnings(&pool).await.len(), 1);
    agents.clock.advance(TimeDelta::minutes(1));
    agents.push(&token, &cpu_batch(PVE_01, now, 5.0)).await;
    assert_eq!(device_warnings(&pool).await.len(), 2);
}

/// The device name and message of each metrics warning that names a device, oldest first.
async fn device_warnings(pool: &PgPool) -> Vec<(Option<String>, String)> {
    sqlx::query_as(
        "SELECT device_name, message FROM events.event_log \
         WHERE source = 'metrics' AND severity = 'warning' AND device_id IS NOT NULL \
         ORDER BY at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn net_002_an_address_outside_the_permitted_networks_reaches_no_agent_route(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let code = agents.code_for(DOCKER_HOST).await;
    let body = json!({ "code": code }).to_string();
    assert_eq!(
        agents.enrol_body(body, OUTSIDE).await.0,
        StatusCode::FORBIDDEN
    );
    let bearer = format!("Bearer {token}");
    let renewal = agents.renew_raw(Some(&bearer), "{}", OUTSIDE).await;
    assert_eq!(renewal.0, StatusCode::FORBIDDEN);
    // Neither request reached the route: the code and the token are untouched.
    assert_eq!(agents.enrol(&code).await.0, StatusCode::OK);
    assert_eq!(agents.renew(&token).await.0, StatusCode::OK);
    let refusals: Vec<String> =
        sqlx::query_scalar("SELECT message FROM events.event_log WHERE source = 'access'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        refusals.iter().any(|message| message.contains(OUTSIDE)),
        "{refusals:?}"
    );
}
