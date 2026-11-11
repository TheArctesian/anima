//! MET-013, K8 Section 5: a renewed token sits beside the current one, which keeps working
//! until the renewed one is first used; a second renewal before then replaces the unused one;
//! every token works for 365 days.

use axum::http::StatusCode;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use sqlx::PgPool;

use super::harness::{Agents, PERMITTED, PVE_01, refusal};
use crate::hub::start;

fn refused() -> (StatusCode, String) {
    (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"))
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
