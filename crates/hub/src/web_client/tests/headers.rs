//! The K7 static file headers, on files, on `index.html` and on refusals (K7, D-2).

use axum::http::{Method, StatusCode};

use super::{Answer, WebRoot, get, router_at, send};
use crate::access::PublicOrigin;
use crate::web_client::web_client;

/// The policy for `INDEX_HTML` and `https://hub.anima.ing`, written out by hand.
const POLICY: &str = "default-src 'self'; \
    script-src 'self' 'sha256-fDYmfWjvur9AxjhDbjHe6RvExc05MbXhhrbgz2vvy/A=' \
    'sha256-VRcK3v7WYQJL3yHheBe4dsvLYjAYwFTyoPi2odncRio='; \
    connect-src 'self' wss://hub.anima.ing; img-src 'self' data: blob:; \
    style-src 'self' 'unsafe-inline'; font-src 'self'; object-src 'none'; base-uri 'none'; \
    form-action 'none'; frame-ancestors 'none'";
const NO_CACHE: &str = "no-cache";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// The headers every web client answer carries, whatever its status, and its cache rule.
fn assert_k7_headers(answer: &Answer, cache: &str) {
    assert_eq!(answer.header("content-security-policy"), POLICY);
    assert_eq!(answer.header("x-content-type-options"), "nosniff");
    assert_eq!(answer.header("referrer-policy"), "no-referrer");
    assert_eq!(answer.header("cross-origin-opener-policy"), "same-origin");
    assert_eq!(
        answer.header("strict-transport-security"),
        "max-age=31536000"
    );
    assert_eq!(answer.header("cache-control"), cache);
}

#[tokio::test]
async fn d_2_index_html_carries_the_headers_and_no_cache() {
    let web = WebRoot::new();
    for path in ["/", "/index.html"] {
        let answer = get(&web.router(), path).await;
        assert_eq!(answer.status, StatusCode::OK, "{path}");
        assert_eq!(answer.header("content-type"), "text/html; charset=utf-8");
        assert_k7_headers(&answer, NO_CACHE);
    }
}

#[tokio::test]
async fn d_2_an_immutable_asset_is_cached_for_a_year() {
    let answer = get(&WebRoot::new().router(), "/_app/immutable/entry/start.js").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_k7_headers(&answer, IMMUTABLE);
}

#[tokio::test]
async fn d_2_other_files_are_no_cache() {
    let web = WebRoot::new();
    for path in ["/_app/version.json", "/robots.txt"] {
        let answer = get(&web.router(), path).await;
        assert_eq!(answer.status, StatusCode::OK, "{path}");
        assert_k7_headers(&answer, NO_CACHE);
    }
}

#[tokio::test]
async fn d_2_the_index_fallback_for_an_unknown_path_carries_the_headers() {
    let answer = get(&WebRoot::new().router(), "/devices/0190a000/detail").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_k7_headers(&answer, NO_CACHE);
}

#[tokio::test]
async fn sec_001_refusals_carry_the_headers_and_are_never_cached_as_immutable() {
    let web = WebRoot::new();
    let router = web.router();
    for path in ["/_app/immutable/missing.js", "/.env", "/%2e%2e/outside.txt"] {
        let answer = get(&router, path).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{path}");
        assert_k7_headers(&answer, NO_CACHE);
    }
    let post = send(&router, Method::POST, "/_app/immutable/entry/start.js").await;
    assert_eq!(post.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_k7_headers(&post, NO_CACHE);
}

#[tokio::test]
async fn d_2_a_loopback_http_origin_allows_ws_in_connect_src() {
    let web = WebRoot::new();
    let origin = PublicOrigin::new("http://127.0.0.1:8443".to_owned());
    let answer = get(&web_client(&web.root(), &origin), "/").await;
    let policy = answer.header("content-security-policy");
    assert!(
        policy.contains("; connect-src 'self' ws://127.0.0.1:8443; "),
        "{policy}"
    );
}

#[tokio::test]
async fn sec_001_an_origin_that_is_not_origin_text_adds_nothing_to_the_policy() {
    let web = WebRoot::new();
    for text in [
        "https://a; script-src *",
        "ftp://hub.anima.ing",
        "https://a b",
    ] {
        let origin = PublicOrigin::new(text.to_owned());
        let answer = get(&web_client(&web.root(), &origin), "/").await;
        let policy = answer.header("content-security-policy");
        assert!(
            policy.contains("; connect-src 'self'; img-src"),
            "{text}: {policy}"
        );
        assert!(!policy.contains('*'), "{text}: {policy}");
    }
}

#[tokio::test]
async fn d_2_a_page_without_inline_scripts_allows_only_self_scripts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<p>no scripts</p>").unwrap();
    let answer = get(&router_at(dir.path()), "/").await;
    let policy = answer.header("content-security-policy");
    assert!(
        policy.contains("; script-src 'self'; connect-src"),
        "{policy}"
    );
}
