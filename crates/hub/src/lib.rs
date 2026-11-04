//! Hub library face: schema and HTTP router, so integration tests can reach them.

pub mod schema;

use async_graphql_axum::GraphQL;
use axum::{Router, routing::post_service};

pub fn router() -> Router {
    Router::new().route(
        "/graphql",
        post_service(GraphQL::new(schema::build_schema())),
    )
}
