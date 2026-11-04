use std::{collections::BTreeMap, sync::Arc};

use anima_hub::{
    clock::SystemClock,
    config, router,
    schema::{HubSchema, build_schema},
};
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn schema() -> HubSchema {
    let config = config::from_env(&BTreeMap::new()).unwrap();
    build_schema(Arc::new(config), Arc::new(SystemClock))
}

#[tokio::test]
async fn health_query_returns_ok() {
    let response = schema().execute("{ health }").await;
    assert_eq!(response.data, async_graphql::value!({ "health": "ok" }));
}

#[tokio::test]
async fn graphql_endpoint_answers_post_over_http() {
    let request = Request::post("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ health }"}"#))
        .unwrap();
    let response = router(schema()).oneshot(request).await.unwrap();
    assert!(response.status().is_success());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), br#"{"data":{"health":"ok"}}"#);
}
