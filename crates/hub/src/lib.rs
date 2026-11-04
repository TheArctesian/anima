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
pub mod sessions;
pub mod status;

use std::sync::Arc;

use async_graphql_axum::GraphQLResponse;
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{Extensions, HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::post,
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

pub fn router(schema: schema::HubSchema, db: PgPool, clock: Arc<dyn clock::Clock>) -> Router {
    Router::new()
        .route("/graphql", post(graphql))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(Edge { schema, db, clock })
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
        Err(error) => {
            error.log();
            async_graphql::Response::from_errors(vec![error.to_server_error()])
        }
    };
    GraphQLResponse::from(response).into_response()
}
