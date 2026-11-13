//! The web client (D-2, K7, Section 1.3): the static build from `ANIMA_WEB_ROOT`, served as
//! the router's fallback, so every explicit route wins. A leaf: of the hub it imports only
//! `access::PublicOrigin`, and only `lib.rs` uses it.
//!
//! Each answer carries the K7 headers, the 404s and 405s too, so no HTML ever leaves without
//! the CSP. `GET` and `HEAD` only. A path with a segment starting with `.` is 404 (dotfiles,
//! `..`, in any encoding). Under `/_app/` a missing file is 404; elsewhere an unknown path
//! gets `index.html` with 200, for client-side routes. `ServeDir` reads the files (A-1).
//!
//! The root is trusted build output, set by the operator: `ServeDir` follows a symlink in it,
//! even one that points outside the root, so the root must hold no link the operator did not
//! mean to publish (SEC-001).

mod headers;
mod path;
mod scripts;
#[cfg(test)]
mod tests;

use std::{path::Path, sync::Arc};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{IntoResponse as _, Response},
};
use tower::ServiceExt as _;
use tower_http::services::ServeDir;

use crate::fleet::access::PublicOrigin;
use path::Wanted;

/// The page every client-side route loads; its inline scripts give the CSP hashes.
const INDEX: &str = "index.html";
const ALLOWED: &str = "GET, HEAD";

/// What the fallback serves from.
struct Site {
    files: ServeDir,
    /// `index.html` as read at build. `None` when it could not be read: every path is then 404.
    index: Option<Bytes>,
    policy: HeaderValue,
}

/// The web client under `root`, for the router's fallback. Reads `root/index.html` once and
/// allows its inline scripts by hash; `connect-src` allows the WebSocket form of `origin`.
///
/// When `index.html` cannot be read this logs one fixed error and returns a router that
/// answers 404, with the headers, for every path: the hub still serves its API and never
/// serves a page whose scripts its CSP does not name.
pub fn web_client(root: &Path, origin: &PublicOrigin) -> Router {
    let index = match std::fs::read(root.join(INDEX)) {
        Ok(index) => Some(Bytes::from(index)),
        Err(error) => {
            // The kind only: never the path or the OS text (LOG-007).
            tracing::error!(
                domain = "web_client",
                kind = %error.kind(),
                "the web client's index.html cannot be read; every web client path answers 404"
            );
            None
        }
    };
    let hashes = index
        .as_deref()
        .map_or_default(|page| scripts::inline_script_hashes(&String::from_utf8_lossy(page)));
    let site = Site {
        files: ServeDir::new(root).append_index_html_on_directories(false),
        index,
        policy: headers::content_security_policy(&hashes, &origin.websocket()),
    };
    Router::new().fallback(answer).with_state(Arc::new(site))
}

/// Answers one request and sets the K7 headers on the answer.
async fn answer(State(site): State<Arc<Site>>, request: Request) -> Response {
    let wanted = path::wanted(request.uri().path());
    let mut response = respond(&site, wanted, request).await;
    let cache = headers::cache_control(wanted, response.status());
    headers::apply(response.headers_mut(), &site.policy, cache);
    response
}

/// The answer before the K7 headers.
async fn respond(site: &Site, wanted: Wanted, request: Request) -> Response {
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        return (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, ALLOWED)]).into_response();
    }
    let Some(index) = &site.index else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let head = request.method() == Method::HEAD;
    match wanted {
        Wanted::Hidden => StatusCode::NOT_FOUND.into_response(),
        Wanted::Index => page(index, head),
        Wanted::Asset { .. } => file(&site.files, request).await,
        Wanted::Page => {
            let response = file(&site.files, request).await;
            // A name the file system cannot look up (too long, a symlink loop) is a 5xx from
            // `ServeDir`; for a client-side route it is just no file, so the page answers.
            let status = response.status();
            if status == StatusCode::NOT_FOUND || status.is_server_error() {
                page(index, head)
            } else {
                response
            }
        }
    }
}

/// The file `ServeDir` finds for `request`, or its 404.
async fn file(files: &ServeDir, request: Request) -> Response {
    match files.clone().oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(never) => match never {},
    }
}

/// `index.html` with 200; without its body for `HEAD`.
fn page(index: &Bytes, head: bool) -> Response {
    let body = if head {
        Body::empty()
    } else {
        Body::from(index.clone())
    };
    let length = HeaderValue::from(index.len());
    let html = HeaderValue::from_static("text/html; charset=utf-8");
    (
        [
            (header::CONTENT_TYPE, html),
            (header::CONTENT_LENGTH, length),
        ],
        body,
    )
        .into_response()
}
