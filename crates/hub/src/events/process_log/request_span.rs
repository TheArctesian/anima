//! One span and one ID per GraphQL request or subscription (LOG-002).

use std::sync::Arc;

use async_graphql::{
    Response,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextRequest, NextSubscribe},
    futures_util::{
        StreamExt,
        stream::{BoxStream, poll_fn},
    },
};
use axum::http::HeaderValue;
use tracing::{Instrument, Span, info_span};
use uuid::Uuid;

/// Runs each request, and each poll of a subscription stream, inside a `graphql_request` span
/// carrying a `request_id` (UUID v7). A request returns the ID in the `x-request-id` header.
///
/// It never records the query text or the variables: they can hold passwords (LOG-007).
pub struct RequestSpan;

impl ExtensionFactory for RequestSpan {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(Self)
    }
}

fn request_span() -> (Span, String) {
    let request_id = Uuid::now_v7().to_string();
    (info_span!("graphql_request", %request_id), request_id)
}

#[async_graphql::async_trait::async_trait]
impl Extension for RequestSpan {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let (span, request_id) = request_span();
        let mut response = next.run(ctx).instrument(span).await;
        let header = HeaderValue::from_str(&request_id).expect("a UUID is a valid header value");
        response.http_headers.insert("x-request-id", header);
        response
    }

    fn subscribe<'s>(
        &self,
        ctx: &ExtensionContext<'_>,
        stream: BoxStream<'s, Response>,
        next: NextSubscribe<'_>,
    ) -> BoxStream<'s, Response> {
        let (span, _) = request_span();
        let mut inner = next.run(ctx, stream);
        // Every poll runs inside the span, so the resolver's own log lines carry the ID.
        Box::pin(poll_fn(move |cx| {
            let _entered = span.enter();
            inner.poll_next_unpin(cx)
        }))
    }
}
