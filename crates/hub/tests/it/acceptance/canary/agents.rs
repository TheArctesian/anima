//! An agent enrolment code made, used and used again, the agent token it gives, a renewal,
//! a push whose body quotes the token, and a push with the renewed token (MET-012, MET-013, K8,
//! LOG-007).

use axum::http::StatusCode;
use serde_json::{Value, json};

use super::{Secrets, edge::WholeHub};

/// `pve-01` in the `devices` fixture.
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";

/// Enrols an agent for `pve-01` with the operator's `token`, renews its token once, and sends
/// what the hub refuses.
pub async fn enrol(hub: &WholeHub, token: &str, secrets: &mut Secrets) {
    let query = "mutation($id: ID!) { enrolAgent(deviceId: $id) { code expiresAt } }";
    let made = hub.data(token, query, json!({ "id": PVE_01 })).await;
    let code = made["enrolAgent"]["code"].as_str().unwrap().to_owned();
    secrets.keep("agent-code", &code);

    let body = json!({ "code": code }).to_string();
    let enrolled = hub.post("/agent/v1/enrol", &[], body.clone()).await;
    assert_eq!(enrolled.status, StatusCode::OK);
    let agent = token_of(&enrolled.json());
    secrets.keep("agent-token", &agent);
    let again = hub.post("/agent/v1/enrol", &[], body).await;
    assert_eq!(again.status, StatusCode::UNAUTHORIZED, "a code enrols once");

    // A body that is not a batch, quoting the token: its parse error is never logged.
    let bearer = format!("Bearer {agent}");
    let quoted = json!({ "device": PVE_01, "series": agent, "points": [] }).to_string();
    let pushed = hub
        .post("/agent/v1/push", &[("authorization", &bearer)], quoted)
        .await;
    assert_eq!(pushed.status, StatusCode::BAD_REQUEST, "a malformed batch");
    let renewed = hub
        .post(
            "/agent/v1/renew",
            &[("authorization", &bearer)],
            "{}".to_owned(),
        )
        .await;
    assert_eq!(renewed.status, StatusCode::OK);
    let next = token_of(&renewed.json());
    secrets.keep("agent-next-token", &next);
    // The renewed token, sent once on a push with an empty body.
    let bearer = format!("Bearer {next}");
    let pushed = hub
        .post(
            "/agent/v1/push",
            &[("authorization", &bearer)],
            "{}".to_owned(),
        )
        .await;
    assert_eq!(
        pushed.status,
        StatusCode::BAD_REQUEST,
        "the next token is accepted"
    );
}

fn token_of(answer: &Value) -> String {
    answer["token"].as_str().unwrap().to_owned()
}
