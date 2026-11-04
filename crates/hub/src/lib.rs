//! Hub library face: module tree and HTTP router, so integration tests can reach them.

pub mod access;
pub mod clock;
pub mod config;
pub mod credentials;
pub mod devices;
pub mod events;
pub mod ids;
pub mod paging;
pub mod scalars;
pub mod schema;
pub mod status;

use std::sync::Arc;

use async_graphql_axum::GraphQLResponse;
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{Extensions, HeaderMap, StatusCode, header::CONTENT_TYPE},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sqlx::PgPool;

/// Largest `/graphql` request body. A bigger one gets 413 before it is parsed or any token
/// or password is checked (SEC-002).
pub const BODY_LIMIT: usize = 1024 * 1024;

/// What the `/graphql` handler needs besides the request.
#[derive(Clone)]
struct Edge {
    schema: schema::HubSchema,
    db: PgPool,
    clock: Arc<dyn clock::Clock>,
}

/// The hub's HTTP routes: `POST /graphql` and `GET /health`, each request in a logged span.
pub fn router(schema: schema::HubSchema, db: PgPool, clock: Arc<dyn clock::Clock>) -> Router {
    logged(
        Router::new()
            .route("/graphql", post(graphql))
            .route("/health", get(health))
            .layer(DefaultBodyLimit::max(BODY_LIMIT))
            .with_state(Edge { schema, db, clock }),
    )
}

/// Runs every route of `routes` in an `http_request` span that records the route template,
/// never a path value, so `/push/{token}` is logged as `/push/[redacted]` (LOG-007, R-028).
/// Apply it to the whole router after the last route is added; WP-09 widens it when the push
/// router is merged in.
pub(crate) fn logged(routes: Router) -> Router {
    routes.layer(middleware::from_fn(events::process_log::http_request))
}

/// Liveness probe (R-026(4)): no sign-in, no database, body `ok`.
async fn health() -> &'static str {
    "ok"
}

/// Whether the media type is exactly `application/json`, in any case, parameters allowed.
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

/// Reads one JSON request of at most [`BODY_LIMIT`] bytes (no batch, no multipart), resolves
/// the bearer token and the client address once, then runs it (D-6).
async fn graphql(
    State(edge): State<Edge>,
    headers: HeaderMap,
    extensions: Extensions,
    body: Bytes,
) -> Response {
    if !is_json(&headers) {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Send one JSON request.").into_response();
    }
    let Ok(mut request) = serde_json::from_slice::<async_graphql::Request>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            "The request is not a GraphQL request.",
        )
            .into_response();
    };
    if let Some(client) = access::client_addr(&extensions) {
        request = request.data(client);
    }
    let response = match access::identify(&edge.db, &headers, edge.clock.now()).await {
        Ok(Some(identity)) => edge.schema.execute(identity.attach(request)).await,
        Ok(None) => edge.schema.execute(request).await,
        // The schema's boundary never sees this error, so the edge logs its cause chain (H-5).
        Err(error) => {
            let error = error.into_response_error();
            events::process_log::log_failure(&error);
            async_graphql::Response::from_errors(vec![error])
        }
    };
    GraphQLResponse::from(response).into_response()
}
