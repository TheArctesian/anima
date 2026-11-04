//! How a devices failure appears in a GraphQL response (K7, API-010).

use std::sync::Arc;

use async_graphql::{Context, Error as GraphQlError, ErrorExtensions, Result, value};

use super::{Error, FieldError};

const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

/// The GraphQL error with its `extensions.code`; a database failure shows no detail.
///
/// Accepts the error itself or the `Arc` a data loader hands back.
pub(super) fn to_graphql(error: impl Into<Arc<Error>>) -> GraphQlError {
    let error = error.into();
    match &*error {
        Error::NotFound(missing) => coded(format!("No such {missing}."), "NOT_FOUND"),
        Error::Validation(fields) => invalid(fields),
        Error::Database(_) => internal(error),
    }
}

/// The generic `INTERNAL` error: the operator sees no detail. The cause stays as the error's
/// source, which is never serialised, so a log layer can report it.
pub(super) fn internal(cause: impl std::error::Error + Send + Sync + 'static) -> GraphQlError {
    let mut error = GraphQlError::new_with_source(cause);
    INTERNAL_MESSAGE.clone_into(&mut error.message);
    error.extend_with(|_, extensions| extensions.set("code", "INTERNAL"))
}

/// A schema datum a resolver needs is not registered: a wiring mistake, never the operator's.
#[derive(Debug, thiserror::Error)]
#[error("schema data missing: {0}")]
struct MissingData(String);

/// Reads schema data; if it is missing the operator gets `INTERNAL`, not the Rust type name.
pub(super) fn schema_data<'a, T: Send + Sync + 'static>(ctx: &Context<'a>) -> Result<&'a T> {
    ctx.data::<T>()
        .map_err(|missing| internal(MissingData(missing.message)))
}

/// A `VALIDATION` error that lists every field the operator must fix.
pub(super) fn invalid(fields: &[FieldError]) -> GraphQlError {
    let listed: Vec<_> = fields
        .iter()
        .map(|e| value!({ "field": e.field, "message": e.message.clone() }))
        .collect();
    GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
        extensions.set("code", "VALIDATION");
        extensions.set("fields", async_graphql::Value::List(listed));
    })
}

fn coded(message: String, code: &'static str) -> GraphQlError {
    GraphQlError::new(message).extend_with(|_, extensions| extensions.set("code", code))
}
