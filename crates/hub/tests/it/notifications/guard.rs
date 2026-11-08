//! Every channel field needs a signed-in operator (API-003).

use async_graphql::{Request, Variables};
use serde_json::json;
use sqlx::PgPool;

use super::harness::{DELETE, create_query, error_json, schema_for, update_query, webhook};

#[sqlx::test(migrations = "./migrations")]
async fn api_003_channel_fields_without_an_operator_are_unauthenticated(pool: PgPool) {
    let schema = schema_for(&pool);
    let id = "0190a000-0000-7000-8fff-000000000001";
    let operations = [
        ("{ notificationChannels { id } }".to_owned(), json!({})),
        (create_query(), json!({ "input": webhook("Ops") })),
        (update_query(), json!({ "id": id, "input": webhook("Ops") })),
        (DELETE.to_owned(), json!({ "id": id })),
    ];
    for (query, variables) in operations {
        let request = Request::new(&query).variables(Variables::from_json(variables));
        let response = schema.execute(request).await;
        assert_eq!(
            error_json(&response)["extensions"]["code"],
            "UNAUTHENTICATED",
            "{query}"
        );
    }
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications.channels")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}
