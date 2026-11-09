//! Who may use the hub: operator accounts, sign-in (desktop and browser), API tokens and the
//! GraphQL guard (SEC-002, API-003, D-3), and from where: permitted networks and the listening
//! edge (NET-001 to NET-003, SEC-001, SEC-008).

mod browser;
mod builder_tokens;
mod connection;
pub mod graphql;
mod guard;
mod identity;
mod layer;
pub mod listen;
mod login;
mod networks;
mod origin;
mod password;
mod store;
#[cfg(test)]
mod tests;
mod throttle;
mod tokens;
mod totp;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_graphql::{ErrorExtensions as _, Pos, ServerError};
use chrono::{DateTime, TimeDelta, Utc};
use ipnet::IpNet;
use sqlx::PgPool;
use zeroize::Zeroizing;

pub use crate::events::{ClientAddr, Operator};
pub use browser::SameOrigin;
pub use builder_tokens::{IssuedToken, issue_builder_token, revoke_builder_token};
pub use connection::{MESSAGE_LIMIT, SUBPROTOCOL, SocketAccess, accept_socket};
pub use guard::{Authenticated, BUILDER_FIELDS, BuilderSignedIn, SignInRequired};
pub use identity::{Identity, client_addr, identify, identify_connection, operator_for_token};
pub use layer::{Edge, permitted_networks_layer};
pub use origin::PublicOrigin;
pub use password::Password;
pub use tokens::{Scope, TokenKind};

use crate::{
    credentials::{self, Secret, Vault},
    ids::OperatorId,
};

/// Login attempts are kept this long (K2 retention).
const ATTEMPTS_KEPT: TimeDelta = TimeDelta::days(90);

/// An expired API key is kept this long, so the operator still sees why a script stopped
/// (K2 0010-0029, WP-43 A-5).
const EXPIRED_KEYS_KEPT: TimeDelta = TimeDelta::days(30);

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
            Self::Unauthenticated | Self::WrongCredentials | Self::TotpSecret(_) => {
                "UNAUTHENTICATED"
            }
            Self::ReadOnly | Self::OtherOrigin | Self::NotPermitted | Self::OutOfScope(_) => {
                "FORBIDDEN"
            }
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
    fn into_graphql(self) -> async_graphql::Error {
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
    // Write the operator row before deleting tokens: a sign-in issues its token under that
    // row's lock, so the delete then sees it (D-9).
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
    let sealed = vault
        .seal(&secret, totp::context(account.id).as_bytes())
        .map_err(|error| match error {
            credentials::Error::Random(source) => Error::Random(source),
            other => Error::TotpSeal(other),
        })?;
    store::set_totp(&mut *tx, account.id, Some(&sealed.to_bytes())).await?;
    // Row first, tokens second, as in `replace_operator` above (D-9).
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

/// Deletes login attempts older than 90 days, expired login and browser tokens, and API keys
/// 30 days after they expired (K2 retention, WP-43 A-5). A scheduled job calls it daily.
///
/// # Errors
///
/// `Error::Database` if the rows cannot be deleted.
pub async fn prune(db: &PgPool, now: DateTime<Utc>) -> Result<Pruned, Error> {
    let mut conn = db.acquire().await?;
    let cutoffs = store::Cutoffs {
        attempts: now - ATTEMPTS_KEPT,
        tokens: now,
        keys: now - EXPIRED_KEYS_KEPT,
    };
    let (attempts, tokens) = store::prune(&mut conn, &cutoffs).await?;
    Ok(Pruned { attempts, tokens })
}

/// The guarded value: the access state behind a lock (counts, maps and timestamps) stays valid
/// when a panic elsewhere poisoned the lock, so serving goes on.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
