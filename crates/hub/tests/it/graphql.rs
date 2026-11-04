use std::{collections::BTreeMap, sync::Arc};

use anima_hub::{
    clock::{Clock, SystemClock},
    config,
    events::Bus,
    router,
    schema::{HubSchema, build_schema},
};
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// A schema whose pool never connects: these requests are refused before any query.
fn schema() -> (HubSchema, sqlx::PgPool, Arc<dyn Clock>) {
    let config = config::from_env(&BTreeMap::new()).unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let schema = build_schema(Arc::new(config), clock.clone(), pool.clone(), Bus::new(1));
    (schema, pool, clock)
}

#[tokio::test]
async fn api_003_health_query_needs_a_token() {
    let response = schema().0.execute("{ health }").await;
    assert_eq!(response.data, async_graphql::Value::Null);
    assert_eq!(response.errors[0].message, "Sign in to continue.");
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
    assert_eq!(body.as_ref(), br#"{"data":{"__typename":"Query"}}"#);
}
