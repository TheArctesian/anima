//! Every root field and both subscriptions refuse a request without a signed-in operator, and
//! store nothing (API-003).

use async_graphql::{Request, Variables, futures_util::StreamExt as _};
use serde_json::json;
use sqlx::PgPool;

use super::harness::{PVE_01, PVE_01_PING, schema_for};

const ID: &str = PVE_01_PING;

fn unsigned(query: &str, variables: serde_json::Value) -> Request {
    Request::new(query).variables(Variables::from_json(variables))
}

fn calls() -> Vec<(&'static str, serde_json::Value)> {
    let config = json!({ "tcp": { "port": 22 } });
    vec![
        (
            "query($id: ID!) { check(id: $id) { id } }",
            json!({ "id": ID }),
        ),
        (
            "mutation($input: CreateCheckInput!) { createCheck(input: $input) { id } }",
            json!({ "input": { "deviceId": PVE_01, "name": "x", "config": config } }),
        ),
        (
            "mutation($id: ID!, $input: UpdateCheckInput!) { updateCheck(id: $id, input: $input) { id } }",
            json!({ "id": ID, "input": { "name": "y" } }),
        ),
        (
            "mutation($id: ID!) { deleteCheck(id: $id) }",
            json!({ "id": ID }),
        ),
        (
            "mutation($id: ID!) { setCheckPaused(id: $id, paused: true) { id } }",
            json!({ "id": ID }),
        ),
    ]
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn api_003_every_check_root_field_refuses_a_request_without_an_operator(pool: PgPool) {
    let schema = schema_for(&pool);
    for (query, variables) in calls() {
        let response = schema.execute(unsigned(query, variables)).await;
        assert_eq!(response.errors.len(), 1, "{query}");
        let error = serde_json::to_value(&response.errors[0]).unwrap();
        assert_eq!(error["extensions"]["code"], "UNAUTHENTICATED", "{query}");
    }
    let state: (i64, String, bool, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM checks.checks), \
                (SELECT name FROM checks.checks WHERE id = $1::text::uuid), \
                (SELECT paused FROM checks.checks WHERE id = $1::text::uuid), \
                (SELECT count(*) FROM events.audit_log)",
    )
    .bind(ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, (28, "Ping".to_owned(), false, 0));
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_both_subscriptions_refuse_a_request_without_an_operator(pool: PgPool) {
    let schema = schema_for(&pool);
    let queries = [
        "subscription { deviceStatusChanged { deviceId } }",
        "subscription { heartbeatAdded { checkId } }",
    ];
    for query in queries {
        let first = schema
            .execute_stream(Request::new(query))
            .next()
            .await
            .unwrap();
        assert_eq!(first.errors.len(), 1, "{query}");
        let error = serde_json::to_value(&first.errors[0]).unwrap();
        assert_eq!(error["extensions"]["code"], "UNAUTHENTICATED", "{query}");
    }
}
