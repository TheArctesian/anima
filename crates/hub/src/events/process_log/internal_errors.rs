//! Keeps internal details out of GraphQL errors (API-010).

use std::sync::Arc;

use async_graphql::{
    Response,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextRequest},
};

const CODE: &str = "code";
const INTERNAL: &str = "INTERNAL";
const GENERIC_MESSAGE: &str = "Something went wrong on the hub.";

/// Turns each resolver error that has no `extensions.code` into `INTERNAL` with a generic
/// message, and writes the original message to the process log at error level.
///
/// Register it after `RequestSpan`, so the log line carries the `request_id`. Errors that
/// carry a code, and errors outside a resolver (parse and validation errors, which name the
/// client's own query), pass unchanged.
pub struct InternalErrors;

impl ExtensionFactory for InternalErrors {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(Self)
    }
}

#[async_graphql::async_trait::async_trait]
impl Extension for InternalErrors {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let mut response = next.run(ctx).await;
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
        response
    }
}
