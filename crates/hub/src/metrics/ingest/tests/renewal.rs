//! K8 Section 5, MET-013: a renewal is stored only while the token it presents still works, even
//! when that token was checked a moment earlier.

use agent_core::wire::AgentToken;
use chrono::TimeDelta;
use sqlx::PgPool;

use super::{database_now, enrolled_pve, token_hashes};
use crate::metrics::ingest::{TOKEN_LIFETIME, renewal::renew};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_013_a_token_at_its_expiry_renews_nothing(pool: PgPool) {
    let now = database_now(&pool).await;
    let current = enrolled_pve(&pool, now, 7).await;
    let expiry = now + TOKEN_LIFETIME;

    let late = renew(&pool, &current, AgentToken::from_random([8; 32]), expiry).await;
    assert!(late.unwrap().is_none());
    assert_eq!(token_hashes(&pool).await.1, None);

    let in_time = expiry - TimeDelta::seconds(1);
    let renewed = renew(&pool, &current, AgentToken::from_random([9; 32]), in_time).await;
    let renewed = renewed.unwrap().unwrap();
    assert_eq!(renewed.expires_at, in_time + TOKEN_LIFETIME);
    assert!(token_hashes(&pool).await.1.is_some());
}
