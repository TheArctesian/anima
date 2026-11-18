//! What `Query.timeline` refuses (API-003, API-006).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{TIMELINE, hour_before, signed_in, timeline};
use crate::hub::{error, unauthenticated};

#[sqlx::test(migrations = "./migrations")]
async fn api_006_a_bad_span_page_size_or_cursor_is_a_validation_error(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let (from, to) = hour_before();
    for bad in [
        json!({ "from": to, "to": from }),
        json!({ "to": from }),
        json!({ "first": 0 }),
        json!({ "first": 201 }),
        json!({ "after": "e1.1904212800123456.0192f5a07c3e7a529d1b3f4e5a6b7c8d" }),
    ] {
        let response = timeline(&hub, &token, bad.clone()).await;
        assert_eq!(error(&response).1, "VALIDATION", "{bad}: {response}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_the_timeline_needs_a_token(pool: PgPool) {
    let (hub, _token) = signed_in(&pool).await;
    let (from, to) = hour_before();
    let response = hub
        .post(None, TIMELINE, json!({ "from": from, "to": to }))
        .await;
    assert_eq!(error(&response), unauthenticated());
    assert_eq!(response["data"], Value::Null);
}
