//! The web client through its router, on a web root in a temporary folder (D-2, K7).

mod headers;
mod path;
mod refusals;
mod scripts;
mod serving;

use std::{fs, path::Path};

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode},
};
use http_body_util::BodyExt as _;
use tempfile::TempDir;
use tower::ServiceExt as _;

use super::web_client;
use crate::fleet::access::PublicOrigin;

/// Two inline scripts (one with attributes) and one loaded by `src`.
const INDEX_HTML: &str = "<!doctype html><html><head>\
    <script src=\"/_app/immutable/entry/start.js\"></script>\
    <script>first()</script>\
    </head><body><script type=\"module\" data-x='a>b'>second()</script></body></html>";
const START_JS: &str = "export const start = 1;";
/// Text that must never reach a client: a dotfile in the root and a file beside the root.
const DOTFILE: &str = "SECRET-IN-DOTFILE";
const OUTSIDE: &str = "SECRET-OUTSIDE-ROOT";

/// A folder holding the web root `web/` and, beside it, `outside.txt`.
struct WebRoot {
    dir: TempDir,
}

impl WebRoot {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let web = dir.path().join("web");
        fs::create_dir_all(web.join("_app/immutable/entry")).unwrap();
        fs::create_dir_all(web.join(".hidden")).unwrap();
        fs::write(web.join("index.html"), INDEX_HTML).unwrap();
        fs::write(web.join("_app/immutable/entry/start.js"), START_JS).unwrap();
        fs::write(web.join("_app/version.json"), "{\"version\":\"1\"}").unwrap();
        fs::write(web.join("robots.txt"), "User-agent: *").unwrap();
        fs::write(web.join(".env"), DOTFILE).unwrap();
        fs::write(web.join(".hidden/file.txt"), DOTFILE).unwrap();
        fs::write(dir.path().join("outside.txt"), OUTSIDE).unwrap();
        Self { dir }
    }

    fn root(&self) -> std::path::PathBuf {
        self.dir.path().join("web")
    }

    fn router(&self) -> Router {
        router_at(&self.root())
    }
}

/// The web client at `root` for the public origin `https://hub.anima.ing`.
fn router_at(root: &Path) -> Router {
    web_client(root, &PublicOrigin::new("https://hub.anima.ing".to_owned()))
}

/// One answer: status, headers and body text.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Answer {
    fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .map_or("", |value| value.to_str().unwrap())
    }
}

async fn send(router: &Router, method: Method, path: &str) -> Answer {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    answer_to(router, request).await
}

/// The answer to a `GET` of `path` with one request header.
async fn get_with(router: &Router, path: &str, name: &str, value: &str) -> Answer {
    let request = Request::get(path)
        .header(name, value)
        .body(Body::empty())
        .unwrap();
    answer_to(router, request).await
}

async fn answer_to(router: &Router, request: Request<Body>) -> Answer {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Answer {
        status,
        headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    }
}

async fn get(router: &Router, path: &str) -> Answer {
    send(router, Method::GET, path).await
}
