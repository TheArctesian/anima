//! Keeps internal details and argument values out of GraphQL errors and the process log
//! (API-010, LOG-007).

use std::sync::Arc;

use async_graphql::{
    PathSegment, Response, ServerError,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextRequest, NextSubscribe},
    futures_util::{StreamExt, stream::BoxStream},
    value,
};

use crate::events::redaction::redact_text;

const CODE: &str = "code";
const INTERNAL: &str = "INTERNAL";
const VALIDATION: &str = "VALIDATION";
const GENERIC_MESSAGE: &str = "Something went wrong on the hub.";
const ARGUMENT_MESSAGE: &str = "An argument has the wrong type or shape.";

/// How async-graphql 7.2 begins the message of an argument it could not coerce
/// (`InputValueError::expected_type` and `InputValueError::custom`). Those messages echo the
/// input value, which may be a password. A test against async-graphql's real output pins them.
const ARGUMENT_PREFIXES: [&str; 2] = [r#"Expected input type ""#, r#"Failed to parse ""#];

/// Turns each resolver error that has no `extensions.code` into `INTERNAL` with a generic
/// message, and writes the original message, with secrets masked by the audit redaction rules,
/// to the process log at error level. It covers queries, mutations and every item of a
/// subscription stream.
///
/// A resolver error is recognised by its non-empty `path`: async-graphql sets the path on
/// errors raised in a resolver and leaves it empty on parse and validation errors. Those pass
/// unchanged, as do errors that carry a code.
///
/// An argument async-graphql could not coerce also fails with a path, and its message echoes
/// the value. It becomes `VALIDATION` with a fixed message, and the log keeps only a fixed
/// text, the path and the code (LOG-007, K7).
///
/// Register it after `RequestSpan`, so the log line carries the `request_id`.
pub struct InternalErrors;

impl ExtensionFactory for InternalErrors {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(Self)
    }
}

/// Whether `message` is async-graphql's text for an argument it could not coerce.
pub(super) fn is_argument_error(message: &str) -> bool {
    ARGUMENT_PREFIXES
        .iter()
        .any(|prefix| message.starts_with(prefix))
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
        if is_argument_error(&error.message) {
            tracing::warn!(path = ?error.path, code = VALIDATION, "argument refused");
            refuse_argument(error);
        } else {
            hide_internal(error);
        }
    }
}

/// Logs a resolver fault's message with its secrets masked and answers `INTERNAL` with a
/// generic message (API-010).
fn hide_internal(error: &mut ServerError) {
    let message = redact_text(&error.message);
    tracing::error!(path = ?error.path, "resolver failed: {message}");
    GENERIC_MESSAGE.clone_into(&mut error.message);
    error
        .extensions
        .get_or_insert_with(Default::default)
        .set(CODE, INTERNAL);
}

/// Replaces an argument error's message, which echoes the value, with `VALIDATION` naming the
/// field whose argument failed.
fn refuse_argument(error: &mut ServerError) {
    let field = error.path.iter().rev().find_map(|segment| match segment {
        PathSegment::Field(name) => Some(name.clone()),
        PathSegment::Index(_) => None,
    });
    ARGUMENT_MESSAGE.clone_into(&mut error.message);
    let extensions = error.extensions.get_or_insert_with(Default::default);
    extensions.set(CODE, VALIDATION);
    extensions.set(
        "fields",
        value!([{ "field": field.unwrap_or_default(), "message": ARGUMENT_MESSAGE }]),
    );
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
