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

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Router,
    extract::State,
    http::{Extensions, HeaderMap},
    routing::post,
};
use sqlx::PgPool;

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
        .with_state(Edge { schema, db, clock })
}

/// Resolves the bearer token and the client address once, then runs the request (D-6).
async fn graphql(
    State(edge): State<Edge>,
    headers: HeaderMap,
    extensions: Extensions,
    request: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = request.into_inner();
    if let Some(client) = access::client_addr(&extensions) {
        request = request.data(client);
    }
    match access::identify(&edge.db, &headers, edge.clock.now()).await {
        Ok(Some(identity)) => request = identity.attach(request),
        Ok(None) => {}
        Err(error) => return async_graphql::Response::from_errors(vec![error.to_server_error()]).into(),
    }
    edge.schema.execute(request).await.into()
}
