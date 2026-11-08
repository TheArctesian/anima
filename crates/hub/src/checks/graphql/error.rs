//! How a checks failure appears in a GraphQL response (K7, API-010).

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, coded, internal, invalid},
    checks::Error,
    devices,
};

/// The GraphQL error with its `extensions.code`. Anything the operator cannot act on is
/// `INTERNAL` with no detail; the cause stays as the error's source.
pub(super) fn to_graphql(error: Error) -> GraphQlError {
    match error {
        Error::NotFound(missing) => coded(format!("No such {missing}."), ErrorCode::NotFound),
        Error::Validation(fields) => invalid(&fields),
        other => internal(other),
    }
}

/// The error a data loader hands back, shared between the keys of its batch.
pub(super) fn from_shared(error: std::sync::Arc<Error>) -> GraphQlError {
    match &*error {
        Error::NotFound(missing) => coded(format!("No such {missing}."), ErrorCode::NotFound),
        Error::Validation(fields) => invalid(fields),
        _ => internal(error),
    }
}

/// A device lookup that failed: a missing device is `NOT_FOUND`, the rest `INTERNAL`.
pub(super) fn from_devices(error: devices::Error) -> GraphQlError {
    match error {
        devices::Error::NotFound(missing) => {
            coded(format!("No such {missing}."), ErrorCode::NotFound)
        }
        other => internal(other),
    }
}

#[cfg(test)]
mod tests {
    use async_graphql::Value;

    use super::to_graphql;
    use crate::{api_errors::INTERNAL_MESSAGE, checks::Error, credentials};

    fn code(error: &async_graphql::Error) -> Option<&Value> {
        error.extensions.as_ref()?.get("code")
    }

    #[test]
    fn cq_023_a_failed_random_source_is_internal_with_no_detail() {
        let failed = getrandom::Error::UNSUPPORTED;
        for error in [
            Error::Random(failed),
            Error::Credentials(credentials::Error::Random(failed)),
        ] {
            let answer = to_graphql(error);
            assert_eq!(answer.message, INTERNAL_MESSAGE);
            assert_eq!(code(&answer), Some(&Value::from("INTERNAL")));
        }
    }
}
