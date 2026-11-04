//! Who may use the hub: operator accounts, sign-in, API tokens and the GraphQL guard
//! (SEC-002, API-003).

pub mod graphql;
mod guard;
mod identity;
mod login;
mod password;
mod store;
#[cfg(test)]
mod tests;
mod throttle;
mod tokens;

use async_graphql::{ErrorExtensions as _, Pos, ServerError};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub use crate::events::{ClientAddr, Operator};
pub use guard::{Authenticated, SignInRequired};
pub use identity::{Identity, client_addr, identify, identify_connection};
pub use password::Password;
pub use tokens::Scope;

use crate::ids::OperatorId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sign in to continue.")]
    Unauthenticated,
    /// Also for an unknown username and a throttled attempt, so the answer reveals neither
    /// (D-8).
    #[error("The username or password is wrong.")]
    WrongCredentials,
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: &'static str,
    },
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("Something went wrong on the hub.")]
    Database(#[from] sqlx::Error),
    #[error("Something went wrong on the hub.")]
    Events(#[from] crate::events::Error),
    #[error("Something went wrong on the hub.")]
    Hash(#[from] password::HashError),
    #[error("Something went wrong on the hub.")]
    Task(#[from] tokio::task::JoinError),
    #[error("Something went wrong on the hub.")]
    Random(getrandom::Error),
    #[error("Something went wrong on the hub.")]
    Unavailable,
}

impl Error {
    /// The GraphQL `extensions.code` for this error (K7).
    const fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated | Self::WrongCredentials => "UNAUTHENTICATED",
            Self::Validation { .. } => "VALIDATION",
            Self::Database(_)
            | Self::Events(_)
            | Self::Hash(_)
            | Self::Task(_)
            | Self::Random(_)
            | Self::Unavailable => "INTERNAL",
        }
    }

    /// The GraphQL error the operator sees: the plain message and the K7 code, nothing else.
    fn to_graphql(&self) -> async_graphql::Error {
        let code = self.code();
        let field = match self {
            Self::Validation { field, message } => Some((*field, *message)),
            _ => None,
        };
        async_graphql::Error::new(self.to_string()).extend_with(|_, extensions| {
            extensions.set("code", code);
            if let Some((field, message)) = field {
                let entry = async_graphql::value!([{ "field": field, "message": message }]);
                extensions.set("fields", entry);
            }
        })
    }

    /// The error as a response error, for an edge that fails before execution.
    #[must_use]
    pub fn to_server_error(&self) -> ServerError {
        self.to_graphql().into_server_error(Pos::default())
    }
}

/// Used by the tunnel: resolve a bearer token to an operator.
///
/// # Errors
///
/// `Error::Unauthenticated` for a malformed, unknown or expired token; `Error::Database` if
/// it cannot be looked up.
pub async fn operator_for_token(
    db: &PgPool,
    token: &str,
    now: DateTime<Utc>,
) -> Result<Operator, Error> {
    let identity = identity::resolve(db, token, now)
        .await?
        .ok_or(Error::Unauthenticated)?;
    Ok(identity.operator().clone())
}

/// Creates the operator, or replaces its password, for `anima-hub operator set` (D-9).
/// Replacing a password deletes the operator's tokens, so every client signs in again.
///
/// The username is 1 to 64 ASCII letters, digits, `.`, `_` or `-`; the password at least 12
/// characters and at most 1024 bytes.
///
/// # Errors
///
/// `Error::Validation` for a username or password outside those rules; `Error::Database` if
/// the account cannot be written.
pub async fn set_operator(
    db: &PgPool,
    username: &str,
    password: Password,
    now: DateTime<Utc>,
) -> Result<OperatorId, Error> {
    if !login::possible_username(username) {
        return Err(Error::Validation {
            field: "username",
            message: "Use 1 to 64 letters, digits, dots, dashes or underscores.",
        });
    }
    if password.chars() < password::MIN_PASSWORD_CHARS
        || password.as_bytes().len() > password::MAX_PASSWORD_LEN
    {
        return Err(Error::Validation {
            field: "password",
            message: "Use a password of at least 12 characters and at most 1024 bytes.",
        });
    }
    let salt = login::random::<{ password::SALT_LEN }>()?;
    let hash = tokio::task::spawn_blocking(move || password::hash(&password, &salt)).await??;
    let mut tx = db.begin().await?;
    let id = store::put_operator(&mut tx, username, &hash, now).await?;
    store::delete_operator_tokens(&mut tx, id).await?;
    tx.commit().await?;
    Ok(id)
}
