//! How a build failure appears in a GraphQL response (K7, API-010).

use std::str::FromStr;

use async_graphql::{Error as GraphQlError, ID, Result};

use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid},
    fleet::builds::{Error, Missing},
};

/// The GraphQL error with its `extensions.code`. The message names the kind of thing that is
/// missing, never the id, a token or log text; a hub fault shows no detail.
pub(super) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => coded(not_found_message(missing), ErrorCode::NotFound),
        Error::Validation(fields) => invalid(&fields),
        Error::Conflict(message) => coded(message, ErrorCode::Conflict),
        Error::Forbidden(message) => coded(message, ErrorCode::Forbidden),
        Error::Database(_) | Error::Unreadable | Error::Access(_) | Error::Events(_) => {
            internal(error)
        }
    }
}

const fn not_found_message(missing: Missing) -> &'static str {
    match missing {
        Missing::Job(_) => "No such build job.",
        Missing::Builder(_) => "No such builder.",
    }
}

/// The id an argument names, or a `VALIDATION` error for `field`.
pub(super) fn parse_id<T: FromStr>(id: &ID, field: &'static str) -> Result<T> {
    id.parse()
        .map_err(|_| invalid(&[FieldError::new(field, "That id is not valid.")]))
}
