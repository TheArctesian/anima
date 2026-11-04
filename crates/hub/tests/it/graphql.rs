use std::sync::Arc;

use anima_hub::{
    clock::{Clock, SystemClock},
    events::Bus,
    router,
    schema::{HubSchema, build_schema},
};
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

use crate::hub::dependencies;

/// A schema whose pool never connects: these requests are refused or fail validation before
/// any query.
fn schema() -> (HubSchema, PgPool, Arc<dyn Clock>) {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let schema = build_schema(dependencies(&pool, clock.clone(), Bus::new(1)));
    (schema, pool, clock)
}

#[tokio::test]
async fn api_003_health_query_needs_a_token() {
    let response = schema().0.execute("{ health }").await;
    assert_eq!(response.data, async_graphql::Value::Null);
    assert_eq!(response.errors[0].message, "Sign in to continue.");
}

#[tokio::test]
async fn api_007_a_signed_out_typo_gets_no_field_suggestion() {
    let typo = "mutation { login(input: { username: \"a\", password: \"b\" }) { tokn } }";
    let response = schema().0.execute(typo).await;
    let [error] = response.errors.as_slice() else {
        panic!("expected one error, got {:?}", response.errors);
    };
    assert!(error.message.contains("Unknown field"), "{}", error.message);
    assert!(!error.message.contains("Did you mean"), "{}", error.message);
}

#[tokio::test]
async fn graphql_endpoint_answers_post_over_http() {
    let request = Request::post("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ __typename }"}"#))
        .unwrap();
    let (schema, pool, clock) = schema();
    let response = router(schema, pool, clock).oneshot(request).await.unwrap();
    assert!(response.status().is_success());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body["errors"][0]["extensions"]["code"], "UNAUTHENTICATED",
        "{body}"
    );
}
