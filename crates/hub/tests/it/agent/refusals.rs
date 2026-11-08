//! K8 Sections 3 and 6, CR-15, NET-002: every enrol refusal and every token refusal on renew
//! reaches the event log as a warning with its code and address, never the code or token; one
//! with no valid credential at most once a minute per address. An address outside the
//! permitted networks reaches neither route.

use axum::http::StatusCode;
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::harness::{
    Agents, DOCKER_HOST, NEIGHBOUR, OUTSIDE, PERMITTED, PVE_01, metrics_events, refusal,
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
async fn k8_a_refusal_with_a_valid_token_is_always_written_with_its_device(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let bearer = format!("Bearer {token}");
    for _ in 0..2 {
        let answer = agents.renew_raw(Some(&bearer), "[]", PERMITTED).await;
        assert_eq!(answer, (StatusCode::BAD_REQUEST, refusal("MALFORMED")));
    }
    let written: Vec<(Option<String>, String)> = sqlx::query_as(
        "SELECT device_name, message FROM events.event_log \
         WHERE source = 'metrics' AND severity = 'warning'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let expected = (
        Some("pve-01".to_owned()),
        "Refused an agent token renewal from 100.64.0.7: MALFORMED.".to_owned(),
    );
    assert_eq!(written, [expected.clone(), expected]);
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
