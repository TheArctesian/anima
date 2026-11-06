//! Every sessions root field needs a signed-in operator (API-003, R-022).

use async_graphql::{Request, Variables};
use serde_json::json;
use sqlx::PgPool;

use super::{
    PVE_01, Row,
    harness::{error_json, schema_for, signed_in_until},
    insert,
    starting::{display, ready, session_count},
};

const QUERY: &str = "{ sessions { edges { node { id deviceName } } } }";

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_003_sessions_needs_a_signed_in_operator(pool: PgPool) {
    insert(&pool, &Row::live(1, PVE_01, 0)).await.unwrap();
    let schema = schema_for(&pool);

    // No identity, and an identity that expired before the schema's clock.
    for request in [Request::new(QUERY), signed_in_until(QUERY, 0)] {
        let response = schema.execute(request).await;
        let error = error_json(&response);
        assert_eq!(error["extensions"]["code"], "UNAUTHENTICATED", "{error}");
        assert!(
            !response
                .data
                .into_json()
                .unwrap()
                .to_string()
                .contains("pve-01")
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_003_start_and_end_session_need_a_signed_in_operator(pool: PgPool) {
    ready(&pool, &[PVE_01]).await;
    insert(&pool, &Row::live(1, PVE_01, 0)).await.unwrap();
    let schema = schema_for(&pool);
    let start = "mutation($input: StartSessionInput!) { startSession(input: $input) { token } }";
    let end = "mutation($id: ID!) { endSession(id: $id) { endReason } }";
    let variables = json!({
        "input": { "deviceId": PVE_01, "protocol": "SSH", "display": display() },
        "id": super::session_id(1).to_string(),
    });
    for query in [start, end] {
        for request in [Request::new(query), signed_in_until(query, 0)] {
            let request = request.variables(Variables::from_json(variables.clone()));
            let error = error_json(&schema.execute(request).await);
            assert_eq!(error["extensions"]["code"], "UNAUTHENTICATED", "{error}");
        }
    }
    assert_eq!(session_count(&pool).await, 1, "no session started");
    let live: Option<String> =
        sqlx::query_scalar("select end_reason from sessions.sessions where id = $1")
            .bind(super::session_id(1))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(live, None, "the session was not ended");
}
