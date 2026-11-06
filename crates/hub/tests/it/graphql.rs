use std::sync::Arc;

use anima_hub::{
    clock::{Clock, SystemClock},
    events::Bus,
    router,
    schema::{HubSchema, build_schema},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

use crate::hub::{Hub, dependencies, set_operator, start};

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

/// Sends `request` to the hub's router and returns the status and the body text.
async fn send(request: Request<Body>) -> (StatusCode, String) {
    let (schema, pool, clock) = schema();
    let response = router(schema, pool, clock).oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn r_026_health_answers_ok_over_get_without_a_token_or_a_database() {
    let get = Request::get("/health").body(Body::empty()).unwrap();
    assert_eq!(send(get).await, (StatusCode::OK, "ok".to_owned()));
    let post = Request::post("/health").body(Body::empty()).unwrap();
    assert_eq!(send(post).await.0, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn api_003_t3_signed_out_documents_that_fail_validation_get_one_unauthenticated() {
    let login = r#"login(input: { username: "operator", password: "guess" }) { token }"#;
    let documents = [
        "{ __typename } fragment F on Query { nosuch }".to_owned(),
        "{ __typename } fragment F on NoSuchType { id }".to_owned(),
        "query($x: NoSuchType) { __typename }".to_owned(),
        "query @nosuch { __typename }".to_owned(),
        format!("mutation {{ {login} }} fragment F on Operator {{ nosuch }}"),
        // A field typo inside an allowed `login` names a type too.
        "mutation { login(input: { username: \"a\", password: \"b\" }) { tokn } }".to_owned(),
        // So do the existence checks a valid name passes: same answer as an unknown one.
        "{ __typename } fragment F on Device { id }".to_owned(),
        "query($x: Device) { __typename }".to_owned(),
    ];
    // The refusal of a document that names a field: what nothing must tell apart.
    let refused = serde_json::to_value(schema().0.execute("{ me { id } }").await).unwrap();
    assert_eq!(
        refused["errors"][0]["extensions"],
        json!({ "code": "UNAUTHENTICATED" })
    );
    for document in &documents {
        let response = schema().0.execute(document.as_str()).await;
        let body = serde_json::to_value(&response).unwrap();
        assert_eq!(body, refused, "{document}");
        assert_eq!(body["data"], json!(null));
        assert_eq!(body["errors"].as_array().map(Vec::len), Some(1));
    }
    // T8 round 2: an `operationName` the document lacks, and a syntax error.
    let named = async_graphql::Request::new("query A { __typename }")
        .operation_name("hunter2-correct-horse");
    let unparsed = async_graphql::Request::new("{ __typename ");
    for request in [named, unparsed] {
        let body = serde_json::to_value(schema().0.execute(request).await).unwrap();
        assert_eq!(body, refused);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn api_007_a_signed_in_typo_gets_no_field_suggestion(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let response = hub
        .post(Some(&token), "{ me { usernam } }", json!({}))
        .await;
    let message = response["errors"][0]["message"].as_str().unwrap();
    assert!(message.contains("Unknown field"), "{response}");
    assert!(!message.contains("Did you mean"), "{response}");
}

#[tokio::test]
async fn graphql_endpoint_answers_post_over_http() {
    let request = Request::post("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ __typename }"}"#))
        .unwrap();
    let (status, body) = send(request).await;
    assert!(status.is_success());
    assert_eq!(body, r#"{"data":{"__typename":"Query"}}"#);
}

/// A CORS preflight for `POST /graphql` from `origin`, as a web view sends it.
fn preflight(origin: &str) -> Request<Body> {
    Request::options("/graphql")
        .header("origin", origin)
        .header("access-control-request-method", "POST")
        .header(
            "access-control-request-headers",
            "authorization,content-type",
        )
        .body(Body::empty())
        .unwrap()
}

/// The `access-control-allow-origin` answer to `request`, if any.
async fn allowed_origin(request: Request<Body>) -> Option<String> {
    let (schema, pool, clock) = schema();
    let response = router(schema, pool, clock).oneshot(request).await.unwrap();
    response
        .headers()
        .get("access-control-allow-origin")
        .map(|value| value.to_str().unwrap().to_owned())
}

#[tokio::test]
async fn the_desktop_web_view_may_post_to_graphql() {
    for origin in anima_hub::CLIENT_ORIGINS {
        assert_eq!(
            allowed_origin(preflight(origin)).await.as_deref(),
            Some(origin)
        );
    }
}

#[tokio::test]
async fn another_origin_gets_no_cors_permission() {
    assert_eq!(allowed_origin(preflight("https://example.com")).await, None);
}

#[tokio::test]
async fn a_cross_origin_post_without_a_token_is_still_refused() {
    let request = Request::post("/graphql")
        .header("origin", "tauri://localhost")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "query": "{ devices { totalCount } }" }).to_string(),
        ))
        .unwrap();
    let (status, body) = send(request).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("UNAUTHENTICATED"), "{body}");
}
