//! How a devices failure appears in a GraphQL response (K7, API-010).

use std::sync::Arc;

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, coded, internal, invalid},
    fleet::devices::Error,
};

/// The GraphQL error with its `extensions.code`; a database failure shows no detail.
///
/// Accepts the error itself or the `Arc` a data loader hands back.
pub(in crate::fleet::devices) fn to_graphql(error: impl Into<Arc<Error>>) -> GraphQlError {
    let error = error.into();
    match &*error {
        Error::NotFound(missing) => coded(format!("No such {missing}."), ErrorCode::NotFound),
        Error::Validation(fields) => invalid(fields),
        Error::Database(_) => internal(error),
    }
}
