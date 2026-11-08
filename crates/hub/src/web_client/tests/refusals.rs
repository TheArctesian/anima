//! What the web client refuses: missing build output, traversal, dotfiles, methods other than
//! `GET` and `HEAD`, and every path when `index.html` cannot be read (SEC-001, K7).

use axum::http::{Method, StatusCode};

use super::{DOTFILE, INDEX_HTML, OUTSIDE, WebRoot, get, router_at, send};

#[tokio::test]
async fn d_2_a_missing_app_file_is_404_never_html() {
    let web = WebRoot::new();
    for path in [
        "/_app/immutable/missing.js",
        "/_app/missing.json",
        "/%5fapp/immutable/missing.js",
        "/_app/immutable/entry/",
    ] {
        let answer = get(&web.router(), path).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(answer.body, "", "{path}");
        assert!(!answer.header("content-type").contains("html"), "{path}");
    }
}

#[tokio::test]
async fn sec_001_traversal_never_leaves_the_root() {
    let web = WebRoot::new();
    let router = web.router();
    // Every spelling is a URI the hub can receive: `get` fails the test on one that is not.
    for path in [
        "/../outside.txt",
        "/%2e%2e/outside.txt",
        "/%2E%2E%2Foutside.txt",
        "/..%2foutside.txt",
        "/a/..%5c..%5coutside.txt",
        "/%2e%2e%5coutside.txt",
        "/..\\outside.txt",
        "/_app/%2e%2e/%2e%2e/outside.txt",
        "//outside.txt",
        "/%2Foutside.txt",
        "/%2F%2E%2E/outside.txt",
    ] {
        let answer = get(&router, path).await;
        let page = answer.status == StatusCode::OK && answer.body == INDEX_HTML;
        assert!(
            answer.status == StatusCode::NOT_FOUND || page,
            "{path}: {}",
            answer.status
        );
        assert!(!answer.body.contains(OUTSIDE), "{path} escaped the root");
    }
}

#[tokio::test]
async fn sec_001_dotfiles_and_dot_folders_are_404() {
    let web = WebRoot::new();
    for path in [
        "/.env",
        "/%2eenv",
        "/.hidden/file.txt",
        "/a/%2Ehidden/file.txt",
        "/.hidden/",
    ] {
        let answer = get(&web.router(), path).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{path}");
        assert!(!answer.body.contains(DOTFILE), "{path}");
    }
}

#[tokio::test]
async fn d_2_a_missing_app_file_behind_extra_slashes_is_404_never_html() {
    let web = WebRoot::new();
    for path in [
        "//_app/immutable/missing.js",
        "/%2F_app/immutable/missing.js",
        "/_app//immutable/missing.js",
    ] {
        let answer = get(&web.router(), path).await;
        assert_eq!(
            (answer.status, answer.body.as_str()),
            (StatusCode::NOT_FOUND, ""),
            "{path}"
        );
    }
}

#[tokio::test]
async fn d_2_methods_other_than_get_and_head_are_405_on_every_path() {
    let web = WebRoot::new();
    for method in [
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::PATCH,
    ] {
        for path in ["/", "/devices", "/_app/immutable/entry/start.js", "/.env"] {
            let answer = send(&web.router(), method.clone(), path).await;
            assert_eq!(
                answer.status,
                StatusCode::METHOD_NOT_ALLOWED,
                "{method} {path}"
            );
            assert_eq!(answer.header("allow"), "GET, HEAD");
        }
    }
}

#[tokio::test]
async fn sec_001_without_a_readable_index_every_path_is_404_with_the_policy() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("robots.txt"), "User-agent: *").unwrap();
    let missing = dir.path().join("no-such-folder");
    for root in [dir.path(), missing.as_path()] {
        let router = router_at(root);
        for path in ["/", "/index.html", "/devices", "/robots.txt", "/_app/x.js"] {
            let answer = get(&router, path).await;
            assert_eq!(
                (answer.status, answer.body.as_str()),
                (StatusCode::NOT_FOUND, "")
            );
            let policy = answer.header("content-security-policy");
            assert!(policy.contains("; script-src 'self'; "), "{policy}");
            assert_eq!(answer.header("x-content-type-options"), "nosniff");
        }
    }
}
