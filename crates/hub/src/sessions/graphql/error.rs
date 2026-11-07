//! How a sessions failure appears in a GraphQL response (K7, API-010).

use async_graphql::{Context, Error as GraphQlError, ErrorExtensions, Result, value};

use crate::sessions::{Error, model::FieldError};

const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

const UNAVAILABLE_MESSAGE: &str = "The hub cannot start a session now. Try again in a moment.";

/// The GraphQL error with its `extensions.code` (K7); a dependency failure shows no detail.
pub(in crate::sessions) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => coded(format!("No such {missing}."), "NOT_FOUND"),
        Error::Validation(fields) => invalid(&fields),
        Error::Conflict(conflict) => coded(conflict.to_string(), "CONFLICT"),
        Error::Unavailable => coded(UNAVAILABLE_MESSAGE.to_owned(), "UNAVAILABLE"),
        Error::Devices(_)
        | Error::Events(_)
        | Error::Credentials(_)
        | Error::Random(_)
        | Error::Database(_) => internal(error),
    }
}

fn coded(message: String, code: &'static str) -> GraphQlError {
    GraphQlError::new(message).extend_with(|_, extensions| extensions.set("code", code))
}

/// The generic `INTERNAL` error: the operator sees no detail. The cause stays as the error's
/// source, which is never serialised, so a log layer can report it.
fn internal(cause: impl std::error::Error + Send + Sync + 'static) -> GraphQlError {
    let mut error = GraphQlError::new_with_source(cause);
    INTERNAL_MESSAGE.clone_into(&mut error.message);
    error.extend_with(|_, extensions| extensions.set("code", "INTERNAL"))
}

/// A schema datum a resolver needs is not registered: a wiring mistake, never the operator's.
#[derive(Debug, thiserror::Error)]
#[error("schema dependency not registered: {0}")]
struct UnregisteredDependency(String);

/// Reads schema data; if it is missing the operator gets `INTERNAL`, not the Rust type name.
pub(in crate::sessions) fn schema_dependency<'a, T: Send + Sync + 'static>(
    ctx: &Context<'a>,
) -> Result<&'a T> {
    ctx.data::<T>()
        .map_err(|missing| internal(UnregisteredDependency(missing.message)))
}

/// A `VALIDATION` error that lists every argument the operator must fix.
pub(in crate::sessions) fn invalid(fields: &[FieldError]) -> GraphQlError {
    let listed: Vec<_> = fields
        .iter()
        .map(|e| value!({ "field": e.field, "message": e.message.clone() }))
        .collect();
    GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
        extensions.set("code", "VALIDATION");
        extensions.set("fields", async_graphql::Value::List(listed));
    })
}
