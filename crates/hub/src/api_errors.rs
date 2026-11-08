//! The K7 error plumbing every domain's `graphql` part shares: the code names, the generic
//! `INTERNAL` answer, the `VALIDATION` answer and the schema reads that fail as `INTERNAL`
//! (K7, API-010, CQ-050, CQ-051). A domain keeps its own `to_graphql(Error)`, the mapping
//! from its error enum.

use std::{fmt, sync::Arc};

use async_graphql::{Context, Error as GraphQlError, ErrorExtensions, Result, value};
use sqlx::{PgPool, Postgres, Transaction};

/// The text of an `INTERNAL` error: the operator sees no detail.
pub const INTERNAL_MESSAGE: &str = "Something went wrong on the hub. Try again.";

/// The `extensions.code` values of K7.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    Unauthenticated,
    Forbidden,
    NotFound,
    Validation,
    Conflict,
    Unavailable,
    Internal,
}

impl ErrorCode {
    /// The exact string on the wire.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
            Self::Validation => "VALIDATION",
            Self::Conflict => "CONFLICT",
            Self::Unavailable => "UNAVAILABLE",
            Self::Internal => "INTERNAL",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One input field the operator must fix. `field` is the GraphQL input field name in
/// camelCase, as the client form names it; nested fields are written `ssh.port`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    pub field: &'static str,
    pub message: String,
}

impl FieldError {
    #[must_use]
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// The names of the fields in error, never their values.
#[must_use]
pub fn field_names(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(|error| error.field)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A GraphQL error with this code and message.
pub fn coded(message: impl Into<String>, code: ErrorCode) -> GraphQlError {
    GraphQlError::new(message).extend_with(|_, extensions| extensions.set("code", code.as_str()))
}

/// The generic `INTERNAL` error: the operator sees no detail. The cause stays as the error's
/// source, which is never serialised. It is stored as `Arc<dyn Error + Send + Sync>`, the
/// type the API boundary reads, so the process log gets its chain (H-5).
pub fn internal(cause: impl std::error::Error + Send + Sync + 'static) -> GraphQlError {
    let cause: Arc<dyn std::error::Error + Send + Sync> = Arc::new(cause);
    let mut error = GraphQlError::new_with_source(cause);
    INTERNAL_MESSAGE.clone_into(&mut error.message);
    error.extend_with(|_, extensions| extensions.set("code", ErrorCode::Internal.as_str()))
}

/// A `VALIDATION` error that lists every field the operator must fix.
#[must_use]
pub fn invalid(fields: &[FieldError]) -> GraphQlError {
    let listed: Vec<_> = fields
        .iter()
        .map(|e| value!({ "field": e.field, "message": e.message.clone() }))
        .collect();
    GraphQlError::new("Some input is not valid.").extend_with(|_, extensions| {
        extensions.set("code", ErrorCode::Validation.as_str());
        extensions.set("fields", async_graphql::Value::List(listed));
    })
}

/// A schema datum a resolver needs is not registered: a wiring mistake, never the operator's.
#[derive(Debug, thiserror::Error)]
#[error("schema dependency not registered: {0}")]
struct UnregisteredDependency(String);

/// Reads schema data; if it is missing the operator gets `INTERNAL`, not the Rust type name.
///
/// # Errors
/// `INTERNAL` when the datum is not registered.
pub fn schema_dependency<'a, T: Send + Sync + 'static>(ctx: &Context<'a>) -> Result<&'a T> {
    ctx.data::<T>()
        .map_err(|missing| internal(UnregisteredDependency(missing.message)))
}

/// A database call of a resolver failed; the cause chain reads `database error: <kind>`.
#[derive(Debug, thiserror::Error)]
#[error("database error")]
struct DatabaseFailure(#[from] sqlx::Error);

/// The database pool the schema was built with.
///
/// # Errors
/// `INTERNAL` when the schema has no pool.
pub fn pool<'a>(ctx: &Context<'a>) -> Result<&'a PgPool> {
    schema_dependency::<PgPool>(ctx)
}

/// The transaction a mutation writes its change, its audit entry and its event in.
///
/// # Errors
/// `INTERNAL` when the schema has no pool or the database refuses.
pub async fn begin(ctx: &Context<'_>) -> Result<Transaction<'static, Postgres>> {
    pool(ctx)?
        .begin()
        .await
        .map_err(|e| internal(DatabaseFailure(e)))
}

/// Commits a mutation's transaction.
///
/// # Errors
/// `INTERNAL` when the database refuses the commit.
pub async fn commit(tx: Transaction<'static, Postgres>) -> Result<()> {
    tx.commit().await.map_err(|e| internal(DatabaseFailure(e)))
}

#[cfg(test)]
mod tests {
    use async_graphql::Value;

    use super::{ErrorCode, FieldError, INTERNAL_MESSAGE, coded, field_names, internal, invalid};

    fn code(error: &async_graphql::Error) -> Option<&Value> {
        error.extensions.as_ref()?.get("code")
    }

    #[test]
    fn k7_codes_keep_their_wire_strings() {
        let wire = [
            (ErrorCode::Unauthenticated, "UNAUTHENTICATED"),
            (ErrorCode::Forbidden, "FORBIDDEN"),
            (ErrorCode::NotFound, "NOT_FOUND"),
            (ErrorCode::Validation, "VALIDATION"),
            (ErrorCode::Conflict, "CONFLICT"),
            (ErrorCode::Unavailable, "UNAVAILABLE"),
            (ErrorCode::Internal, "INTERNAL"),
        ];
        for (code, text) in wire {
            assert_eq!(code.as_str(), text);
            assert_eq!(code.to_string(), text);
        }
    }

    #[test]
    fn api_010_internal_has_a_generic_message_and_keeps_the_cause_as_source() {
        let answer = internal(getrandom::Error::UNSUPPORTED);
        assert_eq!(answer.message, INTERNAL_MESSAGE);
        assert_eq!(code(&answer), Some(&Value::from("INTERNAL")));
        let cause = answer.source.as_ref().and_then(|source| {
            source.downcast_ref::<std::sync::Arc<dyn std::error::Error + Send + Sync>>()
        });
        assert!(cause.is_some(), "the process log reads this source type");
    }

    #[test]
    fn k7_invalid_lists_each_field_with_its_message() {
        let answer = invalid(&[
            FieldError::new("name", "Enter a name."),
            FieldError::new("ssh.port", "Use 1 to 65535."),
        ]);
        assert_eq!(code(&answer), Some(&Value::from("VALIDATION")));
        let fields = answer.extensions.as_ref().and_then(|e| e.get("fields"));
        let expected = async_graphql::value!([
            { "field": "name", "message": "Enter a name." },
            { "field": "ssh.port", "message": "Use 1 to 65535." },
        ]);
        assert_eq!(fields, Some(&expected));
    }

    #[test]
    fn k7_coded_sets_the_message_and_the_code() {
        let answer = coded("No such device.", ErrorCode::NotFound);
        assert_eq!(answer.message, "No such device.");
        assert_eq!(code(&answer), Some(&Value::from("NOT_FOUND")));
    }

    #[test]
    fn field_names_lists_names_and_never_messages() {
        let errors = [
            FieldError::new("name", "secret-looking value"),
            FieldError::new("host", "another"),
        ];
        assert_eq!(field_names(&errors), "name, host");
    }
}
