//! How a maintenance failure appears in a GraphQL response (K7, API-010).

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, coded, internal, invalid},
    fleet::maintenance::Error,
};

/// The GraphQL error with its `extensions.code`; a database failure shows no detail.
pub(super) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(_) => coded("No such maintenance window.", ErrorCode::NotFound),
        Error::Validation(fields) => invalid(&fields),
        Error::Database(_) => internal(error),
    }
}
