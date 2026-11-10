//! MET-013, R-092 D-7: revoking ends the device's token and unused codes at once and leaves the
//! metrics source `AGENT`; deleting the device removes its agent rows (MET-011, R-092 D-1). A
//! push that waited for the device row while its token was replaced or the source switched
//! away is refused (CR-18).

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use super::{
    batches::{cpu_batch, samples_of},
    harness::{Agents, DOCKER_HOST, PVE_01, pushing_now, refusal, rows_of, source},
};
use crate::hub::error;

const REVOKE: &str =
    "mutation($id: ID!) { revokeAgentToken(deviceId: $id) { id name metricsSource } }";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_a_revoked_token_is_refused_and_the_source_stays_agent(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let renewed = agents.renewed(&token).await;
    let other = agents.token_for(DOCKER_HOST).await;
    let unused = agents.code_for(PVE_01).await;

    let revoked = agents.operate(REVOKE, json!({ "id": PVE_01 })).await;
    let device = &revoked["data"]["revokeAgentToken"];
    assert_eq!(
        *device,
        json!({ "id": PVE_01, "name": "pve-01", "metricsSource": "AGENT" }),
        "{revoked}"
    );

    let refused = (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"));
    assert_eq!(agents.renew(&token).await, refused);
    assert_eq!(agents.renew(&renewed).await, refused);
    assert_eq!(
        agents.enrol(&unused).await,
        (StatusCode::UNAUTHORIZED, refusal("ENROLMENT_REFUSED"))
    );
    assert_eq!(source(&pool, PVE_01).await, "agent");
    // Another device's agent is untouched.
    assert_eq!(agents.renew(&other).await.0, StatusCode::OK);

    let audit: Vec<(String, String, serde_json::Value)> = sqlx::query_as(
        "SELECT action, host(client_addr), change FROM events.audit_log \
         WHERE target_type = 'device' AND target_id = $1 ORDER BY at, action",
    )
    .bind(PVE_01)
    .fetch_all(&pool)
    .await
    .unwrap();
    let revocation = audit
        .iter()
        .find(|(action, ..)| action == "delete")
        .unwrap();
    assert_eq!(revocation.1, super::harness::PERMITTED);
    assert_eq!(revocation.2, json!({ "agent": { "tokenRevoked": true } }));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_revoking_needs_a_signed_in_operator_and_a_device(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    let signed_out = agents.graphql(None, REVOKE, json!({ "id": PVE_01 })).await;
    assert_eq!(error(&signed_out).1, "UNAUTHENTICATED");
    // An agent token is no sign-in.
    let as_agent = agents
        .graphql(Some(&token), REVOKE, json!({ "id": PVE_01 }))
        .await;
    assert_eq!(error(&as_agent).1, "UNAUTHENTICATED");
    let unknown = "0190a000-0000-7000-8002-0000000000ff";
    let missing = agents.operate(REVOKE, json!({ "id": unknown })).await;
    assert_eq!(error(&missing).1, "NOT_FOUND");
    assert_eq!(agents.renew(&token).await.0, StatusCode::OK);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_011_deleting_a_device_removes_its_token_and_codes(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let token = agents.token_for(PVE_01).await;
    agents.code_for(PVE_01).await;
    assert_eq!(rows_of(&pool, "agents", PVE_01).await, 1);
    assert_eq!(rows_of(&pool, "enrolments", PVE_01).await, 2);

    let deleted = agents
        .operate(
            "mutation($id: ID!) { deleteDevice(id: $id) }",
            json!({ "id": PVE_01 }),
        )
        .await;
    assert_eq!(deleted["data"]["deleteDevice"], PVE_01, "{deleted}");
    assert_eq!(rows_of(&pool, "agents", PVE_01).await, 0);
    assert_eq!(rows_of(&pool, "enrolments", PVE_01).await, 0);
    assert_eq!(
        agents.renew(&token).await,
        (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED"))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_011_r_092_d_7_the_operator_cannot_choose_the_agent_source(pool: PgPool) {
    let agents = Agents::new(&pool).await;
    let update =
        "mutation($id: ID!) { updateDevice(id: $id, input: { metricsSource: AGENT }) { id } }";
    let refused = agents.operate(update, json!({ "id": PVE_01 })).await;
    assert_eq!(error(&refused).1, "VALIDATION");
    assert_eq!(
        refused["errors"][0]["extensions"]["fields"][0]["field"],
        "metricsSource"
    );
    let create = "mutation($site: ID!) { createDevice(input: { siteId: $site, name: \"agent-box\", \
                  host: \"10.10.0.99\", os: LINUX, metricsSource: AGENT }) { id } }";
    let site = "0190a000-0000-7000-8001-000000000001";
    let refused = agents.operate(create, json!({ "site": site })).await;
    assert_eq!(error(&refused).1, "VALIDATION");
    assert_eq!(source(&pool, PVE_01).await, "none");

    // After enrolment and revocation the operator switches the device back (MET-011).
    let token = agents.token_for(PVE_01).await;
    agents.operate(REVOKE, json!({ "id": PVE_01 })).await;
    let back = "mutation($id: ID!) { updateDevice(id: $id, input: { metricsSource: SSH }) { metricsSource } }";
    let switched = agents.operate(back, json!({ "id": PVE_01 })).await;
    assert_eq!(
        switched["data"]["updateDevice"]["metricsSource"], "SSH",
        "{switched}"
    );
    assert_eq!(agents.renew(&token).await.0, StatusCode::UNAUTHORIZED);
}

/// Waits until a statement of this test's database waits for a lock.
async fn until_a_push_waits(pool: &PgPool) {
    for _ in 0..1000 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_catalog.pg_stat_activity \
             WHERE wait_event_type = 'Lock' AND datname = pg_catalog.current_database()",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if waiting > 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("no push waited for the device row");
}

/// A push checked before `change` commits, held on the device row until it has: refused
/// `TOKEN_REFUSED` with nothing stored, since the batch checks again under the lock.
async fn push_across(pool: &PgPool, change: &str) {
    let now = pushing_now(pool).await;
    let agents = Agents::at(pool, now).await;
    let token = agents.token_for(PVE_01).await;
    let mut operator = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM devices.devices WHERE id = $1::uuid FOR UPDATE")
        .bind(PVE_01)
        .execute(&mut *operator)
        .await
        .unwrap();
    let pushed = cpu_batch(PVE_01, now, 5.0);
    let (answer, ()) = tokio::join!(agents.push(&token, &pushed), async {
        until_a_push_waits(pool).await;
        sqlx::query(change)
            .bind(PVE_01)
            .execute(&mut *operator)
            .await
            .unwrap();
        operator.commit().await.unwrap();
    });
    assert_eq!(
        answer,
        (StatusCode::UNAUTHORIZED, refusal("TOKEN_REFUSED")),
        "{change}"
    );
    assert_eq!(samples_of(pool, PVE_01).await, 0, "{change}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn cr_18_a_push_cannot_land_after_a_switch_away_from_the_agent(pool: PgPool) {
    push_across(
        &pool,
        "UPDATE devices.devices SET metrics_source = 'ssh' WHERE id = $1::uuid",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn met_013_a_push_cannot_land_after_its_token_is_replaced(pool: PgPool) {
    push_across(
        &pool,
        "UPDATE metrics.agents SET token_sha256 = sha256('another token'::bytea) \
         WHERE device_id = $1::uuid",
    )
    .await;
}
