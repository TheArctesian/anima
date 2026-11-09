//! K8 Sections 2 and 5: the agent token is read from one `Authorization: Bearer` header, and
//! only its canonical spelling; anything else is no credential. A token works until, not at,
//! its expiry, and a renewed token is promoted only while it works (MET-013).

use agent_core::wire::{AgentToken, EnrolmentCode};
use axum::http::{HeaderMap, HeaderValue, header::AUTHORIZATION};
use chrono::TimeDelta;
use sqlx::PgPool;

use super::{database_now, enrolled_pve, pve, token_hashes};
use crate::metrics::ingest::{
    TOKEN_LIFETIME,
    credential::{authenticate, bearer},
    renewal::renew,
};

fn token() -> AgentToken {
    AgentToken::from_random([7; 32])
}

fn headers(values: &[&str]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for value in values {
        headers.append(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
    }
    headers
}

fn read(values: &[&str]) -> Option<[u8; 32]> {
    bearer(&headers(values)).map(|token| token.hash().to_bytes())
}

#[test]
fn met_013_one_bearer_header_with_the_token_is_read_in_any_scheme_case() {
    let token = token();
    let text = token.expose();
    let expected = Some(token.hash().to_bytes());
    assert_eq!(read(&[&format!("Bearer {text}")]), expected);
    assert_eq!(read(&[&format!("bearer {text}")]), expected);
    assert_eq!(read(&[&format!("BEARER  {text} ")]), expected);
}

#[test]
fn met_013_anything_but_one_bearer_agent_token_is_no_credential() {
    let token = token();
    let text = token.expose();
    let code = EnrolmentCode::from_random([7; 16]);
    let api_key = format!("anima_{}", &text["anima_agent_".len()..]);
    for values in [
        vec![],
        vec![text.to_owned()],
        vec![format!("Basic {text}")],
        vec![format!("Bearer{text}")],
        vec![format!("Bearer {text}"), format!("Bearer {text}")],
        vec![format!("Bearer {}", code.expose())],
        vec![format!("Bearer {api_key}")],
        vec![format!("Bearer {text}=")],
        vec![format!("Bearer {}", &text[..text.len() - 1])],
        vec!["Bearer ".to_owned()],
    ] {
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        assert_eq!(read(&values), None, "{}", values.len());
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_013_the_current_token_stops_working_at_its_expiry(pool: PgPool) {
    let now = database_now(&pool).await;
    let token = enrolled_pve(&pool, now, 7).await;
    let expiry = now + TOKEN_LIFETIME;
    let before = authenticate(&pool, &token, expiry - TimeDelta::seconds(1)).await;
    assert_eq!(before.unwrap(), Some(pve()));
    assert_eq!(authenticate(&pool, &token, expiry).await.unwrap(), None);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_013_an_unused_renewed_token_at_its_expiry_is_refused_and_never_promoted(pool: PgPool) {
    let now = database_now(&pool).await;
    let current = enrolled_pve(&pool, now, 7).await;
    let renewed_at = now + TimeDelta::days(30);
    let next = AgentToken::from_random([8; 32]);
    let presented = AgentToken::parse(next.expose()).unwrap();
    renew(&pool, &current, next, renewed_at)
        .await
        .unwrap()
        .unwrap();
    let stored = token_hashes(&pool).await;
    let expiry = renewed_at + TOKEN_LIFETIME;

    assert_eq!(authenticate(&pool, &presented, expiry).await.unwrap(), None);
    assert_eq!(token_hashes(&pool).await, stored);
    // A second earlier its first use promotes it.
    let in_time = authenticate(&pool, &presented, expiry - TimeDelta::seconds(1)).await;
    assert_eq!(in_time.unwrap(), Some(pve()));
    assert_eq!(token_hashes(&pool).await, (stored.1.unwrap(), None));
}
