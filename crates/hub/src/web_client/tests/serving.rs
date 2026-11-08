//! What the web client serves: files from the root and `index.html` for client-side routes
//! (D-2, A-1).

use axum::http::{Method, StatusCode};

use super::{INDEX_HTML, START_JS, WebRoot, get, send};

#[tokio::test]
async fn d_2_a_file_in_the_root_is_served_with_its_content_type() {
    let answer = get(&WebRoot::new().router(), "/_app/immutable/entry/start.js").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body, START_JS);
    assert!(answer.header("content-type").contains("javascript"));
}

#[tokio::test]
async fn d_2_an_unknown_path_outside_app_gets_index_html_with_200() {
    let web = WebRoot::new();
    for path in [
        "/devices",
        "/devices/0190a000/detail?tab=logs",
        "/settings/",
        "/_app",
    ] {
        let answer = get(&web.router(), path).await;
        assert_eq!(
            (answer.status, answer.body.as_str()),
            (StatusCode::OK, INDEX_HTML),
            "{path}"
        );
    }
}

#[tokio::test]
async fn d_2_a_folder_is_never_listed_or_redirected() {
    let answer = get(&WebRoot::new().router(), "/_app/immutable/entry").await;
    assert_eq!(answer.status, StatusCode::NOT_FOUND);
    assert_eq!(answer.header("location"), "");
}

#[tokio::test]
async fn d_2_the_root_serves_index_html() {
    let answer = get(&WebRoot::new().router(), "/").await;
    assert_eq!(
        (answer.status, answer.body.as_str()),
        (StatusCode::OK, INDEX_HTML)
    );
}

#[tokio::test]
async fn d_2_a_name_the_file_system_cannot_look_up_gets_index_html() {
    let web = WebRoot::new();
    let long = "a".repeat(300);
    for path in [format!("/{long}"), format!("/devices/{long}")] {
        let answer = get(&web.router(), &path).await;
        assert_eq!(
            (answer.status, answer.body.as_str()),
            (StatusCode::OK, INDEX_HTML)
        );
        assert!(
            answer
                .header("content-security-policy")
                .contains("script-src 'self' 'sha256-")
        );
        assert_eq!(answer.header("cache-control"), "no-cache");
    }
}

#[tokio::test]
async fn d_2_head_answers_like_get_without_a_body() {
    let web = WebRoot::new();
    for path in ["/", "/devices", "/_app/immutable/entry/start.js"] {
        let head = send(&web.router(), Method::HEAD, path).await;
        let get = get(&web.router(), path).await;
        assert_eq!(head.status, StatusCode::OK, "{path}");
        assert_eq!(head.body, "", "{path}");
        assert_eq!(
            head.header("content-length"),
            get.body.len().to_string(),
            "{path}"
        );
        assert_eq!(
            head.header("content-security-policy"),
            get.header("content-security-policy"),
        );
    }
}

#[tokio::test]
async fn d_2_index_html_is_the_page_read_when_the_web_client_was_built() {
    let web = WebRoot::new();
    let router = web.router();
    std::fs::write(web.root().join("index.html"), "<script>changed()</script>").unwrap();
    for path in [
        "/",
        "/index.html",
        "//index.html",
        "/%2Findex.html",
        "/devices",
    ] {
        assert_eq!(get(&router, path).await.body, INDEX_HTML, "{path}");
    }
}
