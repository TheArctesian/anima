//! Keeps internal details out of GraphQL errors (API-010).

use std::sync::Arc;

use async_graphql::{
    Response,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextRequest, NextSubscribe},
    futures_util::{StreamExt, stream::BoxStream},
};

const CODE: &str = "code";
const INTERNAL: &str = "INTERNAL";
const GENERIC_MESSAGE: &str = "Something went wrong on the hub.";

/// Turns each resolver error that has no `extensions.code` into `INTERNAL` with a generic
/// message, and writes the original message to the process log at error level. It covers
/// queries, mutations and every item of a subscription stream.
///
/// A resolver error is recognised by its non-empty `path`: async-graphql sets the path on
/// errors raised in a resolver and leaves it empty on parse and validation errors. Those pass
/// unchanged, as do errors that carry a code.
///
/// The original message is logged as it is, so a resolver must never put a password, key or
/// token in an error message (LOG-007).
///
/// Register it after `RequestSpan`, so the log line carries the `request_id`.
pub struct InternalErrors;

impl ExtensionFactory for InternalErrors {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(Self)
    }
}

fn mask(response: &mut Response) {
    for error in &mut response.errors {
        let has_code = error
            .extensions
            .as_ref()
            .is_some_and(|extensions| extensions.get(CODE).is_some());
        if error.path.is_empty() || has_code {
            continue;
        }
        tracing::error!(path = ?error.path, "resolver failed: {}", error.message);
        GENERIC_MESSAGE.clone_into(&mut error.message);
        error
            .extensions
            .get_or_insert_with(Default::default)
            .set(CODE, INTERNAL);
    }
}

#[async_graphql::async_trait::async_trait]
impl Extension for InternalErrors {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let mut response = next.run(ctx).await;
        mask(&mut response);
        response
    }

    fn subscribe<'s>(
        &self,
        ctx: &ExtensionContext<'_>,
        stream: BoxStream<'s, Response>,
        next: NextSubscribe<'_>,
    ) -> BoxStream<'s, Response> {
        next.run(ctx, stream)
            .map(|mut response| {
                mask(&mut response);
                response
            })
            .boxed()
    }
}
