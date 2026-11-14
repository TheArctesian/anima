//! Live reads run from JSON fixtures under `tests/fixtures`, shaped like GitHub's replies.

use source_exchange::{Body, Request, Response};

mod checks;
mod issues;
mod names;
mod paging;
mod pulls;
mod runs;

pub(super) fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap()
}

pub(super) fn ok(body: &[u8]) -> Response<'_> {
    Response {
        status: 200,
        body: Body::Read(body),
    }
}

/// The path and query of `request` as one string, as the engine would send them.
pub(super) fn path_and_query(request: &Request) -> String {
    let query: Vec<String> = request
        .query
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    if query.is_empty() {
        request.path.clone()
    } else {
        format!("{}?{}", request.path, query.join("&"))
    }
}
