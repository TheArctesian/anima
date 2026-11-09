//! MET-013, K8 Section 3: a revoke that commits after an agent's code was redeemed, and before
//! its token is stored, stops that enrolment.

use agent_core::wire::{AgentToken, EnrolmentCode};
use sqlx::PgPool;

use super::{database_now, pve};
use crate::{
    events::Bus,
    metrics::ingest::{
        credential::authenticate,
        enrolment::{enrol, redeem},
        make_code, revoke,
    },
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_013_a_revoke_after_the_code_was_redeemed_stops_the_enrolment(pool: PgPool) {
    let now = database_now(&pool).await;
    let code = EnrolmentCode::from_random([5; 16]);
    let presented = EnrolmentCode::parse(code.expose()).unwrap();
    let mut conn = pool.acquire().await.unwrap();
    make_code(&mut conn, pve(), code, now).await.unwrap();
    drop(conn);
    let redeemed = redeem(&pool, &presented, now).await.unwrap().unwrap();

    let mut revoking = pool.begin().await.unwrap();
    assert!(!revoke(&mut revoking, pve()).await.unwrap());
    revoking.commit().await.unwrap();

    let token = AgentToken::from_random([7; 32]);
    let agent = AgentToken::parse(token.expose()).unwrap();
    let enrolled = enrol(&pool, &Bus::new(4), (redeemed, token), now).await;
    assert!(enrolled.unwrap().is_none());
    assert_eq!(authenticate(&pool, &agent, now).await.unwrap(), None);
    let source: String =
        sqlx::query_scalar("SELECT metrics_source FROM devices.devices WHERE id = $1")
            .bind(pve())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(source, "none");
}
