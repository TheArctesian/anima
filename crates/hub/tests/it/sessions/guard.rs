//! Every sessions root field needs a signed-in operator (API-003, R-022).

use async_graphql::Request;
use sqlx::PgPool;

use super::{
    PVE_01, Row,
    harness::{error_json, schema_for, signed_in_until},
    insert,
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
