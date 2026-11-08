//! Both retention fields need a signed-in operator (API-003).

use async_graphql::Request;
use sqlx::PgPool;

use super::harness::{error_json, schema_for};

#[sqlx::test(migrations = "./migrations")]
async fn api_003_retention_without_an_operator_is_unauthenticated_and_changes_nothing(
    pool: PgPool,
) {
    let schema = schema_for(&pool);
    let operations = [
        ("retention", "{ retention { subject } }"),
        (
            "setRetention",
            "mutation { setRetention(subject: EVENTS, days: 120) { days } }",
        ),
    ];
    for (name, query) in operations {
        let response = schema.execute(Request::new(query)).await;
        assert_eq!(
            error_json(&response)["extensions"]["code"],
            "UNAUTHENTICATED",
            "{name}"
        );
    }
    let days: i32 =
        sqlx::query_scalar("SELECT days FROM retention.periods WHERE subject = 'events'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(days, 90);
}
