//! A builder token reaches only the builder fields, from the address it enrolled from; an
//! operator token never reaches them (BLD-022 to BLD-024, API-003, D-1).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use super::harness::{claim, create_job, data, enrol, operator};
use crate::hub::{Hub, error, start, unauthenticated};

const BUILDER_ONLY: &str = "A builder token may only claim and report build jobs.";
const NOT_BUILDER: &str = "Only a builder token may do this.";

fn forbidden(message: &str) -> (String, String) {
    (message.to_owned(), "FORBIDDEN".to_owned())
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_a_builder_token_is_refused_on_every_operator_field(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let builder = enrol(&hub, &operator, "pop-os").await;
    for query in [
        "{ builders { id } }",
        "{ me { id } }",
        "{ devices { id } }",
        "{ releases { edges { cursor } } }",
        "mutation { revokeBuilder(id: \"0190a000-0000-7000-8009-000000000001\") }",
        "mutation { claimBuildJob(platform: { os: LINUX, arch: X86_64 }) { id } \
         logout }",
        "subscription { buildJobUpdated { job { id } } }",
    ] {
        let response = hub.post(Some(&builder.token), query, json!({})).await;
        assert_eq!(error(&response), forbidden(BUILDER_ONLY), "{query}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_022_an_operator_token_is_refused_on_every_builder_field(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    create_job(&hub, &operator, "0.1.12").await;
    let response = claim(&hub, &operator).await;
    assert_eq!(error(&response), forbidden(NOT_BUILDER));
    let report = "mutation { reportBuildJob(id: \"0190a000-0000-7000-8009-000000000001\", \
                  logLines: []) { id } }";
    let response = hub.post(Some(&operator), report, json!({})).await;
    assert_eq!(error(&response), forbidden(NOT_BUILDER));
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_024_a_builder_token_from_another_address_is_refused(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    create_job(&hub, &operator, "0.1.12").await;
    let builder = enrol(&hub, &operator, "pop-os").await;
    let elsewhere = Hub::from_client(&pool, start(), "100.64.0.9");
    let response = claim(&elsewhere, &builder.token).await;
    assert_eq!(error(&response).1, "FORBIDDEN", "{response}");
    let queued: String = sqlx::query_scalar("SELECT state FROM builds.jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(queued, "queued");
    assert_eq!(
        data(claim(&hub, &builder.token).await)["claimBuildJob"]["state"],
        "CLAIMED"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_023_a_revoked_builder_token_stops_working(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let builder = enrol(&hub, &operator, "pop-os").await;
    let revoke = "mutation($id: ID!) { revokeBuilder(id: $id) }";
    let revoked = data(
        hub.post(Some(&operator), revoke, json!({ "id": builder.id }))
            .await,
    );
    assert_eq!(revoked["revokeBuilder"], builder.id.as_str());
    let response = claim(&hub, &builder.token).await;
    assert_eq!(error(&response), unauthenticated());
    let listed = data(
        hub.post(Some(&operator), "{ builders { id } }", json!({}))
            .await,
    );
    assert_eq!(listed["builders"], json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_023_the_builder_token_is_kept_only_as_its_hash_for_180_days(pool: PgPool) {
    let operator = operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let builder = enrol(&hub, &operator, "pop-os").await;
    let stored: (Vec<u8>, String, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT t.token_hash, t.scope, t.expires_at FROM access.api_tokens t \
         JOIN builds.builders b ON b.token_id = t.id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let bytes = URL_SAFE_NO_PAD.decode(&builder.token).unwrap();
    assert_eq!(stored.0, Sha256::digest(&bytes).to_vec());
    assert_eq!(stored.1, "builder");
    assert_eq!(stored.2, start() + chrono::TimeDelta::days(180));
    let listed = data(
        hub.post(
            Some(&operator),
            "{ builders { id clientAddr online } }",
            json!({}),
        )
        .await,
    );
    assert_eq!(
        listed["builders"],
        json!([{ "id": builder.id, "clientAddr": "100.64.0.7", "online": false }])
    );
}
