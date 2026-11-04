//! One span and one access line per HTTP request, naming the route and never a value in the
//! path (LOG-002, LOG-007, R-028).

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use tracing::{Instrument as _, info_span};

/// What the log records for a request that matched no route: its path is the sender's text.
const NO_ROUTE: &str = "[no route]";
const REDACTED: &str = "[redacted]";

/// axum middleware: runs the request in an `http_request` span with the method and the
/// logged path, and writes one `request served` line with the status.
///
/// The path is the matched route's template with every parameter replaced, so
/// `GET /push/{token}`, where the token is the only authentication (R-028), is recorded as
/// `/push/[redacted]`. The request's own path and query are never recorded. Add it with
/// `Router::layer` after every route, so each route's template is known.
pub async fn http_request(request: Request, next: Next) -> Response {
    let path = logged_path(
        request
            .extensions()
            .get::<MatchedPath>()
            .map(MatchedPath::as_str),
    );
    let span = info_span!("http_request", method = %request.method(), path);
    async move {
        let response = next.run(request).await;
        tracing::info!(status = response.status().as_u16(), "request served");
        response
    }
    .instrument(span)
    .await
}

/// The route template with each `{parameter}` or `{*rest}` segment replaced by
/// `[redacted]`; a request outside every route is `[no route]`.
pub(super) fn logged_path(template: Option<&str>) -> String {
    let Some(template) = template else {
        return NO_ROUTE.to_owned();
    };
    template
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') {
                REDACTED
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}
