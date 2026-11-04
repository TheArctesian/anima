//! Hub library face: schema and HTTP router, so integration tests can reach them.

pub mod schema;

use async_graphql_axum::GraphQL;
use axum::{Router, routing::post_service};

pub fn router() -> Router {
    let graphql = GraphQL::new(schema::build_schema());
    Router::new().route(
        "/graphql",
        post_service(graphql.clone()).get_service(graphql),
    )
}
