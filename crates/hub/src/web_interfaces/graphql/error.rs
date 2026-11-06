//! How a web interface failure appears in a GraphQL response (K7, API-010).

use async_graphql::{Context, Error as GraphQlError, ErrorExtensions, Result, value};

use crate::web_interfaces::{Error, FieldError, Missing};

const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

/// The GraphQL error with its `extensions.code`. The message names the kind of thing that is
/// missing, never the id or any input; a database failure shows no detail.
pub(super) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => {
            GraphQlError::new(not_found_message(missing)).extend_with(|_, extensions| {
                extensions.set("code", "NOT_FOUND");
            })
        }
        Error::Validation(fields) => invalid(&fields),
        Error::Database(_) | Error::Unreadable | Error::Checks(_) => internal(error),
    }
}

const fn not_found_message(missing: Missing) -> &'static str {
    match missing {
        Missing::Interface(_) => "No such web interface.",
        Missing::Device(_) => "No such device.",
        Missing::Check(_) => "No such HTTP check.",
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
#[error("schema dependency not registered: {0}")]
struct UnregisteredDependency(String);

/// Reads schema data; if it is missing the operator gets `INTERNAL`, not the Rust type name.
pub(super) fn schema_dependency<'a, T: Send + Sync + 'static>(ctx: &Context<'a>) -> Result<&'a T> {
    ctx.data::<T>()
        .map_err(|missing| internal(UnregisteredDependency(missing.message)))
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
