//! How a trust failure appears in a GraphQL response (K7, API-010).

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, coded, internal},
    fleet::trust::Error,
};

/// The GraphQL error with its `extensions.code` (K7); a dependency failure shows no detail.
pub(in crate::fleet::trust) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(device) => coded(format!("No such device {device}."), ErrorCode::NotFound),
        Error::Conflict(conflict) => coded(conflict.to_string(), ErrorCode::Conflict),
        Error::Devices(_) | Error::Events(_) | Error::Database(_) => internal(error),
    }
}
