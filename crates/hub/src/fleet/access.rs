//! Who may use the hub: operator accounts, sign-in (desktop and browser), API tokens, API keys
//! for scripts and the GraphQL guard (SEC-002, API-003, INT-003, D-3, D-23), and from where:
//! permitted networks and the listening edge (NET-001 to NET-003, SEC-001, SEC-008).

mod actor;
mod bearer;
mod browser;
mod builder_tokens;
mod connection;
mod error;
pub mod graphql;
mod guard;
mod identity;
mod keys;
mod layer;
pub mod listen;
mod login;
mod networks;
mod origin;
mod password;
mod rebinding;
mod store;
#[cfg(test)]
mod tests;
mod throttle;
mod tokens;
mod totp;

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{Mutex, MutexGuard, PoisonError},
};

use chrono::{DateTime, TimeDelta, Utc};
use ipnet::IpNet;
use sqlx::PgPool;
use zeroize::Zeroizing;

pub use crate::fleet::events::{ClientAddr, Operator};
pub use actor::{ACTOR_HEADER, Declaration, MadeFor};
pub use bearer::bearer_layer;
pub use browser::{SameOrigin, cookie_value};
pub use builder_tokens::{IssuedToken, issue_builder_token, revoke_builder_token};
pub use connection::{MESSAGE_LIMIT, SUBPROTOCOL, SocketAccess, accept_socket};
pub use error::Error;
pub use guard::{Authenticated, BUILDER_FIELDS, BuilderSignedIn, SignInRequired, operator_only};
pub use identity::{Identity, client_addr, identify, identify_connection, operator_for_token};
pub use layer::{Edge, permitted_networks_layer};
pub use origin::PublicOrigin;
pub use password::Password;
pub use rebinding::loopback_names_layer;
pub use tokens::{Scope, TokenKind};

use crate::{
    fleet::{
        credentials::{self, Secret, Vault},
        events::{self, AuditAction, AuditTarget, NewAudit},
    },
    ids::OperatorId,
};

/// Login attempts are kept this long (K2 retention).
const ATTEMPTS_KEPT: TimeDelta = TimeDelta::days(90);

/// An expired API key is kept this long, so the operator still sees why a script stopped
/// (K2 0010-0029, WP-43 A-5).
const EXPIRED_KEYS_KEPT: TimeDelta = TimeDelta::days(30);

/// The label of the networks that come from `ANIMA_PERMITTED_NETWORKS` (D-10).
const CONFIGURED_NETWORKS: &str = "ANIMA_PERMITTED_NETWORKS";

/// Sets the one operator account (OD-2) for `anima-hub operator set` (D-9): creates it, or
/// replaces its username and password. In the same transaction it deletes all its tokens, so
/// every client signs in again (builders go with their tokens; the caller audits them,
/// BLD-025), and clears the login attempts of the old and new username,
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
    let hash = hash_password(password).await?;
    let mut tx = db.begin().await?;
    let (id, action) = replace_account(&mut tx, username, &hash, now).await?;
    let change = serde_json::json!({ "username": username, "reason": "operator set" });
    events::audit(&mut tx, now, account_audit(id, action, change)).await?;
    tx.commit().await?;
    Ok(id)
}

/// What `operator create` read (FRN-004): the username, the password, the TOTP secret that
/// `operator totp --new` showed and a code of it. No `Debug`: it holds secrets (LOG-007).
/// The secret and the code are wiped from memory on drop.
pub struct NewOperator {
    pub username: String,
    pub password: Password,
    pub secret: Zeroizing<String>,
    pub code: Zeroizing<String>,
}

/// Creates or replaces the one operator account with a password and a TOTP secret together,
/// for `anima-hub operator create` (FRN-004, OD-2): the first run and the recovery on the hub
/// computer. Nothing is saved unless the credentials pass [`check_credentials`], the secret is
/// 32 base32 characters and the code is the secret's code at `now`, with the drift sign-in
/// accepts. Then one transaction writes the account as [`set_operator`] does (tokens and
/// login attempts deleted), seals the secret for the operator (R-010) and marks the code's
/// step used, so `login` refuses the same code (SEC-002). So the account never exists without
/// its second factor.
///
/// # Errors
///
/// `Error::Validation` for a refused username, password, secret or code;
/// `Error::Random`, `Error::TotpSeal` or `Error::Database` if the account cannot be written.
pub async fn create_operator(
    db: &PgPool,
    vault: &Vault,
    new: NewOperator,
    now: DateTime<Utc>,
) -> Result<OperatorId, Error> {
    let NewOperator {
        username,
        password,
        secret,
        code,
    } = new;
    check_credentials(&username, &password)?;
    let secret = totp::decode_secret(&secret).ok_or(Error::Validation {
        field: "secret",
        message: "The TOTP secret must be the 32 base32 characters that --new printed.",
    })?;
    let step = totp::accepted_step(&secret, &code, now, None).ok_or(Error::Validation {
        field: "code",
        message: "The code does not match the secret now. Nothing was saved.",
    })?;
    let hash = hash_password(password).await?;
    // Every write below is in this one transaction: a failure at any step leaves no account
    // and no change to an existing one (tested in `tests/it/totp/create.rs`).
    let mut tx = db.begin().await?;
    let (id, action) = replace_account(&mut tx, &username, &hash, now).await?;
    let sealed = seal_totp(vault, id, &Secret::new(secret.to_vec()))?;
    store::set_totp(&mut *tx, id, Some(&sealed), Some(step)).await?;
    let change = serde_json::json!({
        "username": username,
        "totp": true,
        "reason": "operator create",
    });
    events::audit(&mut tx, now, account_audit(id, action, change)).await?;
    tx.commit().await?;
    Ok(id)
}

/// The audit entry of a change that the hub's own commands made to the operator account, on
/// the hub computer (LOG-005): the hub is the actor and this computer the client. `change`
/// names what changed, never a password or a secret (LOG-007).
fn account_audit(id: OperatorId, action: AuditAction, change: serde_json::Value) -> NewAudit {
    NewAudit {
        operator: Operator::hub(),
        action,
        target: AuditTarget::Operator(id),
        client: ClientAddr(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        change,
    }
}

/// The PHC hash of `password` with a new salt, made off the async executor.
async fn hash_password(password: Password) -> Result<String, Error> {
    let salt = login::random::<{ password::SALT_LEN }>()?;
    Ok(tokio::task::spawn_blocking(move || password::hash(&password, &salt)).await??)
}

/// Makes the one account have this username and hash, deletes all its tokens and clears the
/// login attempts of the old and new username, in the caller's transaction. Returns its id and
/// whether the account was created or updated.
async fn replace_account(
    tx: &mut sqlx::PgConnection,
    username: &str,
    hash: &str,
    now: DateTime<Utc>,
) -> Result<(OperatorId, AuditAction), Error> {
    let (id, previous) = store::replace_operator(tx, username, hash, now).await?;
    // Write the operator row before deleting tokens: a sign-in issues its token under that
    // row's lock, so the delete then sees it (D-9).
    store::delete_operator_tokens(tx, id).await?;
    let mut usernames = vec![username];
    usernames.extend(previous.as_deref());
    store::delete_attempts_of(tx, &usernames).await?;
    let action = if previous.is_some() {
        AuditAction::Update
    } else {
        AuditAction::Create
    };
    Ok((id, action))
}

/// What `operator show` prints (FRN-003): the one operator's username and whether it has
/// enrolled TOTP. It holds no secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorStatus {
    pub name: String,
    pub totp: bool,
}

/// The one operator's status for `anima-hub operator show`, or `None` before there is one.
///
/// # Errors
///
/// `Error::Database` if the account cannot be read.
pub async fn operator_status(db: &PgPool) -> Result<Option<OperatorStatus>, Error> {
    let found = store::the_operator(db).await?;
    Ok(found.map(|(name, totp)| OperatorStatus { name, totp }))
}

/// A TOTP secret that `operator totp <username> --new` shows and does not save (FRN-004):
/// the secret in base32 and its `otpauth://` URI. No `Debug`: both hold the secret
/// (LOG-007). Wiped from memory on drop.
pub struct NewTotp {
    secret: Zeroizing<String>,
    uri: Zeroizing<String>,
}

impl NewTotp {
    /// The secret in base32, as `operator create` reads it back.
    #[must_use]
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// The URI an authenticator app enrols from.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

/// Makes a TOTP secret with `username` as the label of its URI, and saves nothing (FRN-004):
/// no operator needs to exist. [`create_operator`] saves it.
///
/// # Errors
///
/// `Error::Validation` for a username `operator set` would refuse; `Error::Random` if the
/// secret cannot be made.
pub fn new_totp(username: &str) -> Result<NewTotp, Error> {
    if !login::possible_username(username) {
        return Err(BAD_USERNAME);
    }
    let secret = Zeroizing::new(login::random::<{ totp::SECRET_LEN }>()?);
    Ok(NewTotp {
        secret: totp::base32(secret.as_slice()),
        uri: totp::otpauth_uri(secret.as_slice(), username),
    })
}

/// Seals a TOTP secret for the operator `id` (R-010): the id, which a rename keeps, binds it.
fn seal_totp(vault: &Vault, id: OperatorId, secret: &Secret) -> Result<Vec<u8>, Error> {
    let sealed = vault
        .seal(secret, totp::context(id).as_bytes())
        .map_err(|error| match error {
            credentials::Error::Random(source) => Error::Random(source),
            other => Error::TotpSeal(other),
        })?;
    Ok(sealed.to_bytes())
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
    let sealed = seal_totp(vault, account.id, &secret)?;
    store::set_totp(&mut *tx, account.id, Some(&sealed), None).await?;
    // Row first, tokens second, as in `replace_operator` above (D-9).
    store::delete_operator_tokens(&mut tx, account.id).await?;
    tx.commit().await?;
    Ok(Enrolment(totp::otpauth_uri(
        secret.expose(),
        &account.username,
    )))
}

/// Removes the operator's TOTP secret for `anima-hub operator totp <username> --disable`.
/// In the same transaction it deletes the operator's tokens, as [`enrol_totp`] does, and
/// writes an audit entry (LOG-005). Outside the link mode the password alone then signs in;
/// in the link mode no sign-in passes until a new secret is set (FRN-007).
///
/// # Errors
///
/// `Error::Validation` when no operator has this username; `Error::Database` otherwise.
pub async fn disable_totp(db: &PgPool, username: &str, now: DateTime<Utc>) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    let account = store::account_by_username(&mut *tx, username)
        .await?
        .ok_or(UNKNOWN_OPERATOR)?;
    store::set_totp(&mut *tx, account.id, None, None).await?;
    // Row first, tokens second, as in `replace_operator` (D-9).
    store::delete_operator_tokens(&mut tx, account.id).await?;
    let change = serde_json::json!({
        "username": account.username,
        "totp": false,
        "reason": "operator totp --disable",
    });
    let entry = account_audit(account.id, AuditAction::Update, change);
    events::audit(&mut tx, now, entry).await?;
    tx.commit().await?;
    Ok(())
}

const UNKNOWN_OPERATOR: Error = Error::Validation {
    field: "username",
    message: "No operator has this username.",
};

const BAD_USERNAME: Error = Error::Validation {
    field: "username",
    message: "Use 1 to 64 letters, digits, dots, dashes or underscores.",
};

/// The rules `operator set` applies: a username of 1 to 64 ASCII letters, digits, `.`, `_`
/// or `-`; a password of at least 12 characters and at most 1024 bytes.
fn check_credentials(username: &str, password: &Password) -> Result<(), Error> {
    if !login::possible_username(username) {
        return Err(BAD_USERNAME);
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

/// Deletes login attempts older than 90 days, expired login, browser and builder tokens, and API
/// keys 30 days after they expired (K2 retention, WP-43 A-5). A builder record goes with its
/// token; the caller audits it (`builds::audit_removed`, BLD-025). A scheduled job calls it
/// daily.
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
