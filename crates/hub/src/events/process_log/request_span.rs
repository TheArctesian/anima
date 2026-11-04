//! One span and one ID per GraphQL request (LOG-002).

use std::sync::Arc;

use async_graphql::{
    Response,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextRequest},
};
use axum::http::HeaderValue;
use tracing::{Instrument, info_span};
use uuid::Uuid;

/// Runs each request inside a `graphql_request` span carrying a `request_id` (UUID v7), and
/// returns the ID in the `x-request-id` response header.
///
/// It never records the query text or the variables: they can hold passwords (LOG-007).
pub struct RequestSpan;

impl ExtensionFactory for RequestSpan {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(Self)
    }
}

#[async_graphql::async_trait::async_trait]
impl Extension for RequestSpan {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let request_id = Uuid::now_v7().to_string();
        let span = info_span!("graphql_request", %request_id);
        let mut response = next.run(ctx).instrument(span).await;
        if let Ok(value) = HeaderValue::from_str(&request_id) {
            response.http_headers.insert("x-request-id", value);
        }
        response
    }
}
