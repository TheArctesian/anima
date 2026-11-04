//! Who may use the hub: operator accounts, sign-in, API tokens and the GraphQL guard
//! (SEC-002, API-003), and from where: permitted networks and the listening edge (NET-001 to
//! NET-003, SEC-001, SEC-008).

pub mod graphql;
mod guard;
mod identity;
mod layer;
pub mod listen;
mod login;
mod networks;
mod password;
mod store;
#[cfg(test)]
mod tests;
mod throttle;
mod tokens;

use async_graphql::{ErrorExtensions as _, Pos, ServerError};
use chrono::{DateTime, TimeDelta, Utc};
use ipnet::IpNet;
use sqlx::PgPool;

pub use crate::events::{ClientAddr, Operator};
pub use guard::{Authenticated, SignInRequired};
pub use identity::{Identity, client_addr, identify, identify_connection};
pub use layer::permitted_networks_layer;
pub use password::Password;
pub use tokens::Scope;

use crate::ids::OperatorId;

/// Login attempts are kept this long (K2 retention).
const ATTEMPTS_KEPT: TimeDelta = TimeDelta::days(90);

/// The label of the networks that come from `ANIMA_PERMITTED_NETWORKS` (D-10).
const CONFIGURED_NETWORKS: &str = "ANIMA_PERMITTED_NETWORKS";

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
            Self::ReadOnly | Self::NotPermitted => "FORBIDDEN",
            Self::Validation { .. } => "VALIDATION",
            Self::Unreachable(_) => "UNAVAILABLE",
            Self::Database(_)
            | Self::Hash(_)
            | Self::Task(_)
            | Self::Random(_)
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
            | Self::NotPermitted
            | Self::Validation { .. } => None,
            Self::Unreachable(_) => Some("database_unreachable"),
            Self::Database(_) => Some("database"),
            Self::Hash(_) => Some("password_hash"),
            Self::Task(_) => Some("blocking_task"),
            Self::Random(_) => Some("random_source"),
            Self::Unavailable => Some("missing_context"),
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

/// Makes the networks from `ANIMA_PERMITTED_NETWORKS` the stored ones with that label, at
/// start (D-10): adds the new ones and removes those no longer listed, so taking a network out
/// of the variable stops permitting it. Networks are stored without host bits, and an
/// IPv4-mapped IPv6 network as IPv4. A network stored under another label is left alone.
///
/// # Errors
///
/// `Error::Validation` for a network inside `172.16.0.0/12`, whose addresses are never
/// taken as clients; `Error::Database` if the networks cannot be written.
pub async fn permit_networks(db: &PgPool, configured: &[IpNet]) -> Result<(), Error> {
    if configured.iter().any(|net| networks::only_untrusted(*net)) {
        return Err(Error::Validation {
            field: CONFIGURED_NETWORKS,
            message: "ANIMA_PERMITTED_NETWORKS may not name a network inside 172.16.0.0/12: \
                      those are container and proxy addresses, never clients.",
        });
    }
    let wanted: Vec<IpNet> = configured
        .iter()
        .copied()
        .map(networks::canonical_net)
        .collect();
    let mut tx = db.begin().await?;
    store::replace_labelled_networks(&mut tx, CONFIGURED_NETWORKS, &wanted).await?;
    tx.commit().await?;
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
