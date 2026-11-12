//! How a sessions failure appears in a GraphQL response (K7, API-010).

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, coded, internal, invalid},
    sessions::Error,
};

const UNAVAILABLE_MESSAGE: &str = "The hub cannot start a session now. Try again in a moment.";

/// The GraphQL error with its `extensions.code` (K7); a dependency failure shows no detail.
pub(in crate::sessions) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => coded(format!("No such {missing}."), ErrorCode::NotFound),
        Error::Validation(fields) => invalid(&fields),
        Error::Conflict(conflict) => coded(conflict.to_string(), ErrorCode::Conflict),
        Error::Unavailable => coded(UNAVAILABLE_MESSAGE.to_owned(), ErrorCode::Unavailable),
        Error::Devices(_)
        | Error::Events(_)
        | Error::Credentials(_)
        | Error::Trust(_)
        | Error::Random(_)
        | Error::Database(_) => internal(error),
    }
}
