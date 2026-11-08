//! How a build failure appears in a GraphQL response (K7, API-010).

use std::str::FromStr;

use async_graphql::{Context, Error as GraphQlError, ErrorExtensions, ID, Result, value};

use crate::builds::{Error, FieldError, Missing};

const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

/// The GraphQL error with its `extensions.code`. The message names the kind of thing that is
/// missing, never the id, a token or log text; a hub fault shows no detail.
pub(super) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => coded(not_found_message(missing), "NOT_FOUND"),
        Error::Validation(fields) => invalid(&fields),
        Error::Conflict(message) => coded(&message, "CONFLICT"),
        Error::Forbidden(message) => coded(message, "FORBIDDEN"),
        Error::Database(_) | Error::Unreadable | Error::Access(_) | Error::Events(_) => {
            internal(error)
        }
    }
}

fn coded(message: &str, code: &'static str) -> GraphQlError {
    GraphQlError::new(message).extend_with(|_, extensions| extensions.set("code", code))
}

const fn not_found_message(missing: Missing) -> &'static str {
    match missing {
        Missing::Job(_) => "No such build job.",
        Missing::Builder(_) => "No such builder.",
    }
}

/// The generic `INTERNAL` error: the caller sees no detail. The cause stays as the error's
/// source, which is never serialised, so the API boundary can write it to the process log.
pub(super) fn internal(cause: impl std::error::Error + Send + Sync + 'static) -> GraphQlError {
    let mut error = GraphQlError::new_with_source(cause);
    INTERNAL_MESSAGE.clone_into(&mut error.message);
    error.extend_with(|_, extensions| extensions.set("code", "INTERNAL"))
}

/// A schema datum a resolver needs is not registered: a wiring mistake, never the caller's.
#[derive(Debug, thiserror::Error)]
#[error("schema dependency not registered: {0}")]
struct UnregisteredDependency(String);

/// Reads schema or request data; if it is missing the caller gets `INTERNAL`, not the Rust
/// type name.
pub(super) fn schema_dependency<'a, T: Send + Sync + 'static>(ctx: &Context<'a>) -> Result<&'a T> {
    ctx.data::<T>()
        .map_err(|missing| internal(UnregisteredDependency(missing.message)))
}

/// A `VALIDATION` error that lists every field to fix.
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

/// The id an argument names, or a `VALIDATION` error for `field`.
pub(super) fn parse_id<T: FromStr>(id: &ID, field: &'static str) -> Result<T> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new(field, "That id is not valid.")]))
}
