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

use async_graphql_axum::GraphQL;
use axum::{Router, routing::post_service};

pub fn router(schema: schema::HubSchema) -> Router {
    Router::new().route("/graphql", post_service(GraphQL::new(schema)))
}
