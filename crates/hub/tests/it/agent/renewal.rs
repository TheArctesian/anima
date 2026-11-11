//! MET-013, K8 Section 5: a renewed token sits beside the current one, which keeps working
//! until the renewed one is first used; a second renewal before then replaces the unused one;
//! every token works for 365 days.

use agent_core::wire::AgentToken;
use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;

use super::{
    batches::{cpu_batch, samples_of},
    harness::{Agents, PERMITTED, PVE_01, pushing_now, refusal},
};
use crate::hub::start;

fn refused() -> (StatusCode, String) {
    (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"))
}

/// A well-formed agent token the hub never issued.
fn never_issued(seed: u8) -> String {
    AgentToken::from_random([seed; 32]).expose().to_owned()
}

/// `token` is refused at the bearer, before the body is read, on push and renewal. The write
/// checks the hash again under the row lock, so a lookup that let `token` through would show
/// only here: its malformed bodies would be answered `MALFORMED`.
async fn refused_at_the_bearer(agents: &Agents, token: &str) {
    let bearer = format!("Bearer {token}");
    let pushed = agents.push_raw(Some(&bearer), "{}", PERMITTED).await;
    assert_eq!(pushed, refused());
    let renewal = agents.renew_raw(Some(&bearer), "[]", PERMITTED).await;
    assert_eq!(renewal, refused());
}

/// pve-01's agent row, every column, and how many samples it has.
async fn agent_rows(pool: &PgPool) -> (String, i64) {
    let row = sqlx::query_scalar(
        "SELECT row_to_json(a)::text FROM metrics.agents a WHERE device_id = $1::uuid",
    )
    .bind(PVE_01)
    .fetch_one(pool)
    .await
    .unwrap();
    (row, samples_of(pool, PVE_01).await)
}

/// T8 S3: the hash of the presented token must match; any other well-formed token is refused,
/// on push and renewal alike, and changes nothing.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_a_token_never_issued_is_refused_while_the_device_is_enrolled(pool: PgPool) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let before = agent_rows(&pool).await;
    let stranger = never_issued(0xa5);
    assert_ne!(stranger, token);
    let batch = cpu_batch(PVE_01, now, 12.5);
    assert_eq!(agents.push(&stranger, &batch).await, refused());
    assert_eq!(agents.renew(&stranger).await, refused());
    refused_at_the_bearer(&agents, &stranger).await;
    assert_eq!(agent_rows(&pool).await, before);
    assert_eq!(agents.push(&token, &batch).await.0, StatusCode::NO_CONTENT);
}

/// T8 S3: while a renewed token waits for its first use, only it and the current token work; a
/// third well-formed token promotes nothing and is refused.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_a_token_neither_current_nor_renewed_is_refused_while_a_renewal_waits(
    pool: PgPool,
) {
    let now = pushing_now(&pool).await;
    let agents = Agents::at(&pool, now).await;
    let current = agents.token_for(PVE_01).await;
    let renewed = agents.renewed(&current).await;
    let before = agent_rows(&pool).await;
    let stranger = never_issued(0x5a);
    assert!(stranger != current && stranger != renewed);
    let batch = cpu_batch(PVE_01, now, 12.5);
    assert_eq!(agents.push(&stranger, &batch).await, refused());
    assert_eq!(agents.renew(&stranger).await, refused());
    refused_at_the_bearer(&agents, &stranger).await;
    assert_eq!(
        agent_rows(&pool).await,
        before,
        "nothing promoted or stored"
    );
    assert_eq!(
        agents.push(&renewed, &batch).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        agents.renew(&current).await,
        refused(),
        "promoted by its first use"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_the_old_token_works_until_the_new_one_is_first_used(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let first = agents.token_for(PVE_01).await;
    let unused = agents.renewed(&first).await;
    // The first token still works while the renewed one is unused (and replaces it).
    let second = agents.renewed(&first).await;
    assert_eq!(agents.renew(&unused).await, refused());
    // The first use of the renewed token ends the one it replaced.
    let third = agents.renewed(&second).await;
    assert_eq!(agents.renew(&first).await, refused());
    // The promoted token is now the current one, beside the newest renewal.
    let fourth = agents.renewed(&second).await;
    assert_eq!(agents.renew(&third).await, refused());
    assert_eq!(agents.renew(&fourth).await.0, StatusCode::OK);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_a_token_works_for_365_days(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    agents
        .clock
        .advance(TimeDelta::days(365) - TimeDelta::seconds(1));
    let (status, text) = agents.renew(&token).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let renewed: Value = serde_json::from_str(&text).unwrap();
    let expires_at: DateTime<Utc> = renewed["expiresAt"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        expires_at,
        start() + TimeDelta::days(730) - TimeDelta::seconds(1)
    );
    let next = renewed["token"].as_str().unwrap().to_owned();

    agents.clock.advance(TimeDelta::seconds(1));
    assert_eq!(agents.renew(&token).await, refused());
    // The renewed token has its own 365 days.
    let newest = agents.renewed(&next).await;
    agents.clock.advance(TimeDelta::days(365));
    assert_eq!(agents.renew(&newest).await, refused());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn k8_renewal_checks_the_bearer_then_the_body(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let bearer = format!("Bearer {token}");
    let malformed = (StatusCode::BAD_REQUEST, refusal("MALFORMED"));
    for body in ["", "[]", "null", "\"{}\"", "{", "{} {}"] {
        let answer = agents.renew_raw(Some(&bearer), body, PERMITTED).await;
        assert_eq!(answer, malformed, "{body}");
    }
    let large = format!(r#"{{"pad":"{}"}}"#, "x".repeat(4096));
    let answer = agents
        .renew_raw(Some(&bearer), large.clone(), PERMITTED)
        .await;
    assert_eq!(
        answer,
        (StatusCode::PAYLOAD_TOO_LARGE, refusal("TOO_LARGE"))
    );
    // Without a valid bearer the body is never read: the answer is the token refusal.
    for authorization in [None, Some("Bearer junk"), Some(&*format!("Basic {token}"))] {
        let answer = agents
            .renew_raw(authorization, large.clone(), PERMITTED)
            .await;
        assert_eq!(answer, refused(), "{authorization:?}");
    }
    let extra = agents
        .renew_raw(Some(&bearer), r#"{"agentVersion":"0.1.0"}"#, PERMITTED)
        .await;
    assert_eq!(extra.0, StatusCode::OK, "{}", extra.1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_two_first_uses_of_a_renewed_token_at_once_are_neither_refused(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let first = agents.token_for(PVE_01).await;
    let renewed = agents.renewed(&first).await;
    let (one, other) = tokio::join!(agents.renew(&renewed), agents.renew(&renewed));
    // Both find the token usable; the device's one turn (MET-017) may send the later one back
    // as `UNAVAILABLE`, which the agent tries again, but never as `TOKEN_REFUSED`.
    let unavailable = (StatusCode::SERVICE_UNAVAILABLE, refusal("UNAVAILABLE"));
    for answer in [&one, &other] {
        assert!(
            answer.0 == StatusCode::OK || *answer == unavailable,
            "{answer:?}"
        );
    }
    assert!(
        one.0 == StatusCode::OK || other.0 == StatusCode::OK,
        "{one:?} {other:?}"
    );
    assert_eq!(agents.renew(&first).await, refused());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_enrolling_again_ends_a_renewed_token_never_used(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let first = agents.token_for(PVE_01).await;
    let renewed = agents.renewed(&first).await;
    let again = agents.token_for(PVE_01).await;
    assert_eq!(agents.renew(&renewed).await, refused());
    assert_eq!(agents.renew(&again).await.0, StatusCode::OK);
}
