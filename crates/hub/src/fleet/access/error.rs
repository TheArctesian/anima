//! What access refuses or fails with, as the GraphQL error code, the client message and the
//! process log name it (API-003, LOG-007).

use std::sync::Arc;

use async_graphql::{ErrorExtensions as _, Pos, ServerError};

use super::password;
use crate::fleet::credentials;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sign in to continue.")]
    Unauthenticated,
    /// Also for an unknown username and a throttled attempt, so the answer reveals neither
    /// (D-8).
    #[error("The username or password is wrong.")]
    WrongCredentials,
    /// A `read` token asked for a mutation.
    #[error("This token may only read.")]
    ReadOnly,
    /// The token's kind may not select a root field of the document, such as an API key
    /// asking to manage keys or start a session (D-23, WP-43 A-6).
    #[error("This token cannot do that. Sign in to the hub to do it.")]
    KindRefused,
    /// `signInBrowser` came from another origin than the hub's own, or with none (A-4).
    #[error("Sign in from the hub's own address.")]
    OtherOrigin,
    /// The token's scope does not cover the field: a builder token asked for an operator
    /// field, or an operator token for a builder field (BLD-022).
    #[error("{0}")]
    OutOfScope(&'static str),
    /// The client address is outside every permitted network (NET-002).
    #[error("This address is not in a permitted network.")]
    NotPermitted,
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: &'static str,
    },
    /// The database cannot be reached (connection, pool or I/O failure).
    #[error("The hub cannot reach its database. Try again shortly.")]
    Unreachable(#[source] sqlx::Error),
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("Something went wrong on the hub.")]
    Database(#[source] sqlx::Error),
    #[error("Something went wrong on the hub.")]
    Hash(#[from] password::HashError),
    #[error("Something went wrong on the hub.")]
    Task(#[from] tokio::task::JoinError),
    #[error("Something went wrong on the hub.")]
    Random(#[source] getrandom::Error),
    #[error("Something went wrong on the hub.")]
    Unavailable,
    /// The stored TOTP secret does not open with this vault and the operator's context. The
    /// operator sees a wrong password, so the answer does not tell that the password was
    /// right; the process log gets the fault.
    #[error("The username or password is wrong.")]
    TotpSecret(credentials::Error),
    /// A new TOTP secret could not be sealed for a reason other than the random source.
    #[error("Something went wrong on the hub.")]
    TotpSeal(credentials::Error),
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut
            | sqlx::Error::PoolClosed
            | sqlx::Error::Io(_)
            | sqlx::Error::Tls(_) => Self::Unreachable(error),
            other => Self::Database(other),
        }
    }
}

impl From<crate::fleet::events::Error> for Error {
    fn from(error: crate::fleet::events::Error) -> Self {
        let crate::fleet::events::Error::Database(error) = error;
        error.into()
    }
}

impl Error {
    /// The GraphQL `extensions.code` for this error (K7).
    pub(super) const fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated | Self::WrongCredentials | Self::TotpSecret(_) => {
                "UNAUTHENTICATED"
            }
            Self::ReadOnly
            | Self::KindRefused
            | Self::OtherOrigin
            | Self::NotPermitted
            | Self::OutOfScope(_) => "FORBIDDEN",
            Self::Validation { .. } => "VALIDATION",
            Self::Unreachable(_) => "UNAVAILABLE",
            Self::Database(_)
            | Self::Hash(_)
            | Self::Task(_)
            | Self::Random(_)
            | Self::TotpSeal(_)
            | Self::Unavailable => "INTERNAL",
        }
    }

    /// For the process log: what failed, as a fixed word. Never the source text, which may
    /// hold SQL or input values (LOG-007, API-010). `None` for refusals the operator caused.
    #[must_use]
    pub const fn fault(&self) -> Option<&'static str> {
        match self {
            Self::Unauthenticated
            | Self::WrongCredentials
            | Self::ReadOnly
            | Self::KindRefused
            | Self::OtherOrigin
            | Self::NotPermitted
            | Self::OutOfScope(_)
            | Self::Validation { .. } => None,
            Self::Unreachable(_) => Some("database_unreachable"),
            Self::Database(_) => Some("database"),
            Self::Hash(_) => Some("password_hash"),
            Self::Task(_) => Some("blocking_task"),
            Self::Random(_) => Some("random_source"),
            Self::Unavailable => Some("missing_context"),
            Self::TotpSecret(_) => Some("totp_secret"),
            Self::TotpSeal(_) => Some("totp_seal"),
        }
    }

    /// Writes a hub fault to the process log, by kind only. Refusals are not logged here.
    pub fn log(&self) {
        if let Some(fault) = self.fault() {
            tracing::error!(domain = "access", fault, "access request failed");
        }
    }

    /// The GraphQL error the operator sees: the plain message and the K7 code, nothing else.
    /// A hub fault keeps a [`Fault`] as its hidden source, which the API boundary writes to
    /// the process log and never sends (H-5).
    pub(super) fn into_graphql(self) -> async_graphql::Error {
        let code = self.code();
        let field = match &self {
            Self::Validation { field, message } => Some((*field, *message)),
            _ => None,
        };
        let mut error = async_graphql::Error::new(self.to_string());
        if let Some(kind) = self.fault() {
            let fault: Arc<dyn std::error::Error + Send + Sync> = Arc::new(Fault {
                kind,
                source: self.into_source(),
            });
            error.source = Some(Arc::new(fault));
        }
        error.extend_with(|_, extensions| {
            extensions.set("code", code);
            if let Some((field, message)) = field {
                let entry = async_graphql::value!([{ "field": field, "message": message }]);
                extensions.set("fields", entry);
            }
        })
    }

    /// The error behind a hub fault, if there is one.
    fn into_source(self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Self::Unreachable(error) | Self::Database(error) => Some(Box::new(error)),
            Self::Hash(error) => Some(Box::new(error)),
            Self::Task(error) => Some(Box::new(error)),
            Self::Random(error) => Some(Box::new(error)),
            Self::TotpSecret(error) | Self::TotpSeal(error) => Some(Box::new(error)),
            Self::Unauthenticated
            | Self::WrongCredentials
            | Self::ReadOnly
            | Self::KindRefused
            | Self::OtherOrigin
            | Self::NotPermitted
            | Self::OutOfScope(_)
            | Self::Validation { .. }
            | Self::Unavailable => None,
        }
    }

    /// The error as a response error, for an edge that fails before execution.
    #[must_use]
    pub fn into_response_error(self) -> ServerError {
        self.into_graphql().into_server_error(Pos::default())
    }
}

/// A hub fault as the process log describes it: the [`Error::fault`] word, then the error
/// behind it. Its message is the word alone, so the chain starts with what failed.
#[derive(Debug, thiserror::Error)]
#[error("{kind}")]
struct Fault {
    kind: &'static str,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
