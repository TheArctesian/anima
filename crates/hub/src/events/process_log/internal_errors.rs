//! Keeps internal details and input values out of GraphQL errors, and puts the cause of every
//! hub fault in the process log (API-010, LOG-007, H-5, H-6).

use std::sync::{Arc, OnceLock};

use async_graphql::{
    Response, ServerError, ServerResult, ValidationResult, Variables,
    extensions::{
        Extension, ExtensionContext, ExtensionFactory, NextParseQuery, NextRequest, NextSubscribe,
        NextValidation,
    },
    futures_util::{StreamExt, stream::BoxStream},
    parser::types::ExecutableDocument,
};

use super::{
    causes::{Cause, chain},
    echoes::{self, Arguments},
};

const CODE: &str = "code";
const INTERNAL: &str = "INTERNAL";
const VALIDATION: &str = "VALIDATION";
const GENERIC_MESSAGE: &str = "Something went wrong on the hub.";

/// The API boundary for errors. For each request it:
///
/// - replaces a syntax error's text, which quotes the query line, with a fixed text and keeps
///   the position (H-6 (e));
/// - replaces a validation error about an argument value, which can quote the value or an
///   unknown input field name, with `VALIDATION` and a fixed text naming the argument path
///   (H-6 (f));
/// - answers an argument async-graphql could not coerce while resolving with `VALIDATION`
///   and a fixed text naming the argument, whatever its code or text. Such an error is told
///   from a resolver's own by its position: it is placed at the argument's value, a resolver
///   error at its field (COULD 1, COULD 2). The log keeps only the path and the argument;
/// - answers every other resolver error without a code, and every `INTERNAL` error, with
///   `INTERNAL` and a generic message, and writes its cause chain to the log at error level
///   (H-5). An error with any other code that carries a hidden cause (an unreachable
///   database) is logged the same way and keeps its message.
///
/// The chain never holds SQL text, values or tokens: links are masked by the audit redaction
/// rules and a database error is logged by SQLSTATE (see `causes`). It covers queries,
/// mutations and every item of a subscription stream.
///
/// Register it after `RequestSpan`, so the log line carries the `request_id`, and before
/// `access::SignInRequired`, so it sees that extension's refusals as coded errors.
pub struct InternalErrors;

impl ExtensionFactory for InternalErrors {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(RequestErrors::default())
    }
}

/// One request's boundary: the argument positions of its document, once it is parsed. A
/// subscription stream parses after `subscribe` returns, so the stream shares the cell.
#[derive(Default)]
struct RequestErrors {
    arguments: Arc<OnceLock<Arguments>>,
}

fn mask(response: &mut Response, arguments: Option<&Arguments>) {
    for error in &mut response.errors {
        let resolved = !error.path.is_empty();
        if resolved && let Some(argument) = echoes::coerced_argument(error, arguments) {
            let argument = argument.to_owned();
            tracing::warn!(path = ?error.path, argument, code = VALIDATION, "argument refused");
            echoes::refuse(error, &argument);
            continue;
        }
        let code = code_of(error);
        if !resolved && code.is_none() && echoes::names_unknown_operation(&error.message) {
            echoes::hide_operation(error);
            continue;
        }
        if (resolved && code.is_none()) || code.as_deref() == Some(INTERNAL) {
            hide_internal(error);
        } else if error.source::<Cause>().is_some() {
            log_cause(error, code.as_deref().unwrap_or_default());
        }
    }
}

/// The error's `extensions.code`, if it is a string.
fn code_of(error: &ServerError) -> Option<String> {
    match error.extensions.as_ref()?.get(CODE)? {
        async_graphql::Value::String(code) => Some(code.clone()),
        _ => None,
    }
}

fn log_cause(error: &ServerError, code: &str) {
    let cause = chain(error);
    tracing::error!(path = ?error.path, code, "request failed: {cause}");
}

/// Writes an error's code and cause chain to the process log, as the boundary does, for an
/// edge that answers before the schema runs (a token lookup that fails; H-5). The chain
/// holds no SQL text, values or tokens.
pub fn log_failure(error: &ServerError) {
    log_cause(error, code_of(error).as_deref().unwrap_or_default());
}

/// Logs the fault's cause chain and answers `INTERNAL` with a generic message (API-010).
fn hide_internal(error: &mut ServerError) {
    log_cause(error, INTERNAL);
    GENERIC_MESSAGE.clone_into(&mut error.message);
    error
        .extensions
        .get_or_insert_with(Default::default)
        .set(CODE, INTERNAL);
}

#[async_graphql::async_trait::async_trait]
impl Extension for RequestErrors {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let mut response = next.run(ctx).await;
        mask(&mut response, self.arguments.get());
        response
    }

    fn subscribe<'s>(
        &self,
        ctx: &ExtensionContext<'_>,
        stream: BoxStream<'s, Response>,
        next: NextSubscribe<'_>,
    ) -> BoxStream<'s, Response> {
        let arguments = Arc::clone(&self.arguments);
        next.run(ctx, stream)
            .map(move |mut response| {
                mask(&mut response, arguments.get());
                response
            })
            .boxed()
    }

    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        match next.run(ctx, query, variables).await {
            Ok(document) => {
                let _ = self.arguments.set(echoes::arguments(&document));
                Ok(document)
            }
            // A refusal from a later extension has a code; a parser error has none.
            Err(mut error) if code_of(&error).is_none() => {
                echoes::hide_syntax(&mut error);
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    async fn validation(
        &self,
        ctx: &ExtensionContext<'_>,
        next: NextValidation<'_>,
    ) -> Result<ValidationResult, Vec<ServerError>> {
        next.run(ctx).await.map_err(|mut errors| {
            for error in &mut errors {
                if echoes::is_value_error(&error.message) {
                    echoes::hide_value(error);
                }
            }
            errors
        })
    }
}
