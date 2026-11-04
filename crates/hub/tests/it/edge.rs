//! The `/graphql` edge before sign-in: body limit, request forms and login bursts (SEC-002).

use std::sync::Arc;

use anima_hub::BODY_LIMIT;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;

use crate::hub::{Hub, USERNAME, set_operator, start};

fn post(content_type: &str, body: impl Into<Body>) -> Request<Body> {
    Request::post("/graphql")
        .header(header::CONTENT_TYPE, content_type)
        .body(body.into())
        .unwrap()
}

fn login_body(username: &str, password: &str) -> String {
    let query = "mutation($i: LoginInput!) { login(input: $i) { token } }";
    let variables = json!({ "i": { "username": username, "password": password } });
    json!({ "query": query, "variables": variables }).to_string()
}

async fn attempts(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_a_body_over_the_limit_is_refused_before_sign_in(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let big = login_body(USERNAME, &"x".repeat(BODY_LIMIT));
    let (status, _) = hub.send(post("application/json", big)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    let at_limit = login_body(USERNAME, &"x".repeat(BODY_LIMIT - 200));
    let (status, _) = hub.send(post("application/json", at_limit)).await;
    assert_eq!(status, StatusCode::OK);
    let rows = attempts(&pool, "select count(*) from access.login_attempts").await;
    assert_eq!(rows, 1, "only the request under the limit reached login");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_only_one_json_request_per_post_is_accepted(pool: PgPool) {
    let hub = Hub::new(&pool, start());
    let multipart = post("multipart/form-data; boundary=x", "--x--");
    assert_eq!(
        hub.send(multipart).await.0,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    let batch = json!([{ "query": "{ __typename }" }, { "query": "{ __typename }" }]);
    let (status, _) = hub.send(post("application/json", batch.to_string())).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let get = Request::get("/graphql?query=%7B__typename%7D")
        .body(Body::empty())
        .unwrap();
    assert_eq!(hub.send(get).await.0, StatusCode::METHOD_NOT_ALLOWED);
    let (status, text) = hub
        .send(post("application/json", r#"{"query":"{ a: __typename }"}"#))
        .await;
    assert_eq!(
        (status, text.as_str()),
        (StatusCode::OK, r#"{"data":{"a":"Query"}}"#)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_a_concurrent_burst_gets_at_most_five_password_checks(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Arc::new(Hub::from_client(&pool, start(), "100.64.0.99"));
    let mut tasks = Vec::new();
    for _ in 0..40 {
        let hub = Arc::clone(&hub);
        tasks.push(tokio::spawn(async move {
            hub.send(post(
                "application/json",
                login_body(USERNAME, "wrong wrong wrong"),
            ))
            .await
        }));
    }
    for task in tasks {
        assert_eq!(task.await.unwrap().0, StatusCode::OK);
    }
    let checked = attempts(
        &pool,
        "select count(*) from access.login_attempts where not refused",
    )
    .await;
    assert!(checked <= 5, "{checked} of 40 attempts were checked");
}
