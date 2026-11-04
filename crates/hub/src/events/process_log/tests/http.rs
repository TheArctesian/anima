//! The HTTP request span names the route and a standard method, never a value in the path or
//! a method the sender made up (LOG-007, R-028).

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    routing::get,
};
use tower::ServiceExt as _;

use super::{
    super::http_span::{logged_method, logged_path},
    capture::capture_log,
};

const TOKEN: &str = "pk_7f3a9c2e41b8d6f0a5e4c3b2a1908f7e";

/// What WP-09 mounts: `GET|POST /push/{token}`, where the token is the only authentication.
fn push_stand_in() -> Router {
    crate::logged(Router::new().route(
        "/push/{token}",
        get(|| async { "ok" }).post(|| async { "ok" }),
    ))
}

async fn send(router: Router, method: &str, uri: &str) -> StatusCode {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    router.oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn log_007_r_028_a_push_request_is_logged_as_push_redacted_never_with_its_token() {
    let (capture, _guard) = capture_log("trace");

    assert_eq!(
        send(push_stand_in(), "GET", &format!("/push/{TOKEN}")).await,
        StatusCode::OK
    );
    assert_eq!(
        send(
            push_stand_in(),
            "POST",
            &format!("/push/{TOKEN}?status=up&msg={TOKEN}")
        )
        .await,
        StatusCode::OK
    );

    let lines = capture.lines();
    let served: Vec<_> = lines
        .iter()
        .filter(|line| line["fields"]["message"] == "request served")
        .collect();
    assert_eq!(served.len(), 2, "{lines:?}");
    for (line, method) in served.iter().zip(["GET", "POST"]) {
        assert_eq!(line["span"]["name"], "http_request", "{line}");
        assert_eq!(line["span"]["path"], "/push/[redacted]", "{line}");
        assert_eq!(line["span"]["method"], method, "{line}");
        assert_eq!(line["fields"]["status"], 200, "{line}");
    }
    assert!(!capture.text().contains(TOKEN), "{}", capture.text());
}

#[tokio::test]
async fn log_007_a_request_outside_every_route_is_logged_without_its_path() {
    let (capture, _guard) = capture_log("trace");

    let status = send(push_stand_in(), "GET", &format!("/pusher/{TOKEN}")).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    let lines = capture.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["span"]["path"], "[no route]");
    assert_eq!(lines[0]["fields"]["status"], 404);
    assert!(!capture.text().contains(TOKEN));
}

#[test]
fn log_007_route_parameters_are_redacted_and_fixed_segments_kept() {
    assert_eq!(logged_path(Some("/push/{token}")), "/push/[redacted]");
    assert_eq!(logged_path(Some("/files/{*rest}")), "/files/[redacted]");
    assert_eq!(logged_path(Some("/graphql")), "/graphql");
    assert_eq!(logged_path(Some("/a/{b}/c")), "/a/[redacted]/c");
    assert_eq!(logged_path(None), "[no route]");
}

#[tokio::test]
async fn log_007_a_custom_method_is_logged_as_other() {
    let (capture, _guard) = capture_log("trace");

    send(push_stand_in(), "Hunter2Secret9xyz", "/push/x").await;

    let lines = capture.lines();
    assert_eq!(lines[0]["span"]["method"], "[other]", "{lines:?}");
    assert!(!capture.text().contains("Hunter2Secret9xyz"));
}

#[test]
fn log_007_standard_methods_are_logged_by_name() {
    for method in [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::HEAD,
        Method::OPTIONS,
        Method::PATCH,
        Method::CONNECT,
        Method::TRACE,
    ] {
        assert_eq!(logged_method(&method), method.as_str());
    }
    let custom = Method::from_bytes(b"PURGE").unwrap();
    assert_eq!(logged_method(&custom), "[other]");
}
