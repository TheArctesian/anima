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
mod totp;

use async_graphql::{ErrorExtensions as _, Pos, ServerError};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use zeroize::Zeroizing;

pub use crate::events::{ClientAddr, Operator};
pub use guard::{Authenticated, SignInRequired};
pub use identity::{Identity, client_addr, identify, identify_connection};
pub use password::Password;
pub use tokens::Scope;

use crate::{
    credentials::{self, Secret, Vault},
    ids::OperatorId,
};

/// Login attempts are kept this long (K2 retention).
const ATTEMPTS_KEPT: TimeDelta = TimeDelta::days(90);

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
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: &'static str,
    },
    /// The database cannot be reached (connection, pool or I/O failure).
    #[error("The hub cannot reach its database. Try again shortly.")]
    Unreachable(sqlx::Error),
    /// The message stays generic: the source error may hold SQL text (API-010).
    #[error("Something went wrong on the hub.")]
    Database(sqlx::Error),
    #[error("Something went wrong on the hub.")]
    Hash(#[from] password::HashError),
    #[error("Something went wrong on the hub.")]
    Task(#[from] tokio::task::JoinError),
    #[error("Something went wrong on the hub.")]
    Random(getrandom::Error),
    #[error("Something went wrong on the hub.")]
    Unavailable,
    /// The stored TOTP secret does not open with this vault and the operator's context.
    #[error("Something went wrong on the hub.")]
    TotpSecret(credentials::Error),
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

impl From<crate::events::Error> for Error {
    fn from(error: crate::events::Error) -> Self {
        let crate::events::Error::Database(error) = error;
        error.into()
    }
}

impl Error {
    /// The GraphQL `extensions.code` for this error (K7).
    const fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated | Self::WrongCredentials => "UNAUTHENTICATED",
            Self::ReadOnly => "FORBIDDEN",
            Self::Validation { .. } => "VALIDATION",
            Self::Unreachable(_) => "UNAVAILABLE",
            Self::Database(_)
            | Self::Hash(_)
            | Self::Task(_)
            | Self::Random(_)
            | Self::Unavailable
            | Self::TotpSecret(_) => "INTERNAL",
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
            | Self::Validation { .. } => None,
            Self::Unreachable(_) => Some("database_unreachable"),
            Self::Database(_) => Some("database"),
            Self::Hash(_) => Some("password_hash"),
            Self::Task(_) => Some("blocking_task"),
            Self::Random(_) => Some("random_source"),
            Self::Unavailable => Some("missing_context"),
            Self::TotpSecret(_) => Some("totp_secret"),
        }
    }

    /// Writes a hub fault to the process log, by kind only. Refusals are not logged here.
    pub fn log(&self) {
        if let Some(fault) = self.fault() {
            tracing::error!(domain = "access", fault, "access request failed");
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

/// Sets the one operator account (OD-2) for `anima-hub operator set` (D-9): creates it, or
/// replaces its username and password. In the same transaction it deletes all its tokens, so
/// every client signs in again, and clears the login attempts of the old and new username,
/// so a locked-out operator gets back in. A second account is never created.
///
/// # Errors
///
/// `Error::Validation` for a username or password outside [`check_credentials`];
/// `Error::Database` if the account cannot be written.
pub async fn set_operator(
    db: &PgPool,
    username: &str,
    password: Password,
    now: DateTime<Utc>,
) -> Result<OperatorId, Error> {
    check_credentials(username, &password)?;
    let salt = login::random::<{ password::SALT_LEN }>()?;
    let hash = tokio::task::spawn_blocking(move || password::hash(&password, &salt)).await??;
    let mut tx = db.begin().await?;
    let (id, previous) = store::replace_operator(&mut tx, username, &hash, now).await?;
    store::delete_operator_tokens(&mut tx, id).await?;
    let mut usernames = vec![username];
    usernames.extend(previous.as_deref());
    store::delete_attempts_of(&mut tx, &usernames).await?;
    tx.commit().await?;
    Ok(id)
}

/// What `operator totp` shows once: the `otpauth://` URI an authenticator app enrols from.
/// No `Debug`: the URI holds the secret (LOG-007). Wiped from memory on drop.
pub struct Enrolment(Zeroizing<String>);

impl Enrolment {
    /// The URI, with the secret in base32. Show it on the operator's terminal only.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.0
    }
}

/// Enrols a new TOTP secret for the operator for `anima-hub operator totp` (SEC-002, D-9):
/// 160 random bits, sealed with the vault for this operator (R-010), replacing any earlier
/// secret. In the same transaction it deletes the operator's tokens, so every client signs
/// in again with the second factor.
///
/// # Errors
///
/// `Error::Validation` when no operator has this username; `Error::Random` or
/// `Error::Database` if the secret cannot be made or stored.
pub async fn enrol_totp(db: &PgPool, vault: &Vault, username: &str) -> Result<Enrolment, Error> {
    let secret = Secret::new(Zeroizing::new(login::random::<{ totp::SECRET_LEN }>()?).to_vec());
    let mut tx = db.begin().await?;
    let account = store::account_by_username(&mut *tx, username)
        .await?
        .ok_or(UNKNOWN_OPERATOR)?;
    let sealed = vault.seal(&secret, totp::context(account.id).as_bytes());
    store::set_totp(&mut *tx, account.id, Some(&sealed.to_bytes())).await?;
    store::delete_operator_tokens(&mut tx, account.id).await?;
    tx.commit().await?;
    Ok(Enrolment(totp::otpauth_uri(
        secret.expose(),
        &account.username,
    )))
}

/// Removes the operator's TOTP secret for `anima-hub operator totp <username> --disable`;
/// the password alone signs in again.
///
/// # Errors
///
/// `Error::Validation` when no operator has this username; `Error::Database` otherwise.
pub async fn disable_totp(db: &PgPool, username: &str) -> Result<(), Error> {
    let account = store::account_by_username(db, username)
        .await?
        .ok_or(UNKNOWN_OPERATOR)?;
    store::set_totp(db, account.id, None).await?;
    Ok(())
}

const UNKNOWN_OPERATOR: Error = Error::Validation {
    field: "username",
    message: "No operator has this username.",
};

/// The rules `operator set` applies: a username of 1 to 64 ASCII letters, digits, `.`, `_`
/// or `-`; a password of at least 12 characters and at most 1024 bytes.
fn check_credentials(username: &str, password: &Password) -> Result<(), Error> {
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
    Ok(())
}

/// What [`prune`] deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pruned {
    pub attempts: u64,
    pub tokens: u64,
}

/// Deletes login attempts older than 90 days and expired tokens (K2 retention). A scheduled
/// job calls it daily.
///
/// # Errors
///
/// `Error::Database` if the rows cannot be deleted.
pub async fn prune(db: &PgPool, now: DateTime<Utc>) -> Result<Pruned, Error> {
    let mut conn = db.acquire().await?;
    let (attempts, tokens) = store::prune(&mut conn, now - ATTEMPTS_KEPT, now).await?;
    Ok(Pruned { attempts, tokens })
}
