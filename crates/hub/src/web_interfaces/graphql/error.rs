//! How a web interface failure appears in a GraphQL response (K7, API-010).

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, coded, internal, invalid},
    web_interfaces::{Error, Missing},
};

/// The GraphQL error with its `extensions.code`. The message names the kind of thing that is
/// missing, never the id or any input; a database failure shows no detail.
pub(super) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => coded(not_found_message(missing), ErrorCode::NotFound),
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
