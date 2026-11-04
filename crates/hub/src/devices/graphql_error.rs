//! How a devices failure appears in a GraphQL response (K7, API-010).

use async_graphql::{Error as GraphQlError, ErrorExtensions, value};

use super::{Error, FieldError};

const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

/// The GraphQL error with its `extensions.code`; a database failure shows no detail.
pub(super) fn to_graphql(error: &Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => coded(format!("No such {missing}."), "NOT_FOUND"),
        Error::Validation(fields) => invalid(fields),
        Error::Database(_) => internal(),
    }
}

/// The generic `INTERNAL` error: no detail reaches the operator.
pub(super) fn internal() -> GraphQlError {
    coded(INTERNAL_MESSAGE.to_owned(), "INTERNAL")
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
