//! Signing in and out: the throttle, the password check, the token and their records
//! (SEC-002, LOG-005, D-7, D-8).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::Semaphore;

use super::{
    ClientAddr, Error, Identity, Operator,
    password::{self, DUMMY_HASH, Password},
    store::{self, Account, AttemptKey, NewToken, Outcome},
    throttle,
    tokens::{LOGIN_LIFETIME, Scope, TOKEN_BYTES, Token},
};
use crate::events::{self, AuditAction, AuditTarget, Bus, NewAudit, NewEvent, Severity, Source};

/// Password checks that run at once. Each takes 19 MiB and a CPU for tens of milliseconds,
/// so a burst of attempts queues here instead of exhausting memory.
static HASHING: Semaphore = Semaphore::const_new(2);

/// Longest username kept in `login_attempts`; longer input is cut there.
const MAX_STORED_USERNAME: usize = 128;

/// A sign-in attempt as received. No `Debug`: it holds the password.
pub(super) struct Attempt {
    pub username: String,
    pub password: Password,
    /// Ignored until TOTP lands (WP-05 T6); an enrolled operator cannot sign in before then.
    pub totp_code: Option<String>,
    pub client: ClientAddr,
    pub at: DateTime<Utc>,
}

/// A successful sign-in: the new token, shown once, and the account.
pub(super) struct SignedIn {
    pub token: Token,
    pub expires_at: DateTime<Utc>,
    pub account: Account,
}

/// Signs the operator in and issues a `write` token valid for 30 days.
///
/// Every attempt is recorded. A wrong password, an unknown username, an enrolled second
/// factor (until T6) and a throttled attempt all give `Error::WrongCredentials`.
pub(super) async fn login(db: &PgPool, bus: &Bus, attempt: Attempt) -> Result<SignedIn, Error> {
    let stored_name = stored_username(&attempt.username);
    let key = AttemptKey {
        id: store::start_attempt(db, attempt.at, &stored_name, attempt.client.0).await?,
        at: attempt.at,
        username: &stored_name,
        client: attempt.client.0,
    };
    let account = {
        let mut conn = db.acquire().await?;
        let (by_username, by_client) =
            store::failures(&mut conn, &key, attempt.at - throttle::LOOKBACK).await?;
        if throttle::refused(&by_username, &by_client, attempt.at) {
            store::finish_attempt(&mut *conn, key.id, Outcome::Refused).await?;
            return Err(Error::WrongCredentials);
        }
        if possible_username(&attempt.username) {
            store::account_by_username(&mut *conn, &attempt.username).await?
        } else {
            None
        }
    };
    let verified = password_matches(attempt.password, account.as_ref()).await?;
    match account {
        Some(account) if verified && second_factor(&account, attempt.totp_code.as_deref()) => {
            succeed(db, bus, &key, account).await
        }
        account => {
            record_failure(db, bus, &key, account.as_ref()).await?;
            Err(Error::WrongCredentials)
        }
    }
}

/// Deletes the identity's token and audits the sign-out.
pub(super) async fn logout(
    db: &PgPool,
    identity: &Identity,
    client: ClientAddr,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    if !store::delete_token(&mut tx, identity.token()).await? {
        return Err(Error::Unauthenticated);
    }
    let entry = audit_entry(identity.operator().clone(), AuditAction::Logout, client);
    events::audit(&mut tx, now, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// Random bytes from the operating system.
pub(super) fn random<const N: usize>() -> Result<[u8; N], Error> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(bytes)
}

/// Checks the password against the account's hash, or against a dummy hash when there is no
/// account, so both take the same time (D-8). Runs off the async executor, two at a time.
async fn password_matches(password: Password, account: Option<&Account>) -> Result<bool, Error> {
    let known = account.is_some();
    let phc = account.map_or_else(|| DUMMY_HASH.to_owned(), |a| a.password_hash.clone());
    // The permit moves into the blocking task, so a cancelled request still holds its place
    // until the hash finishes.
    let permit = HASHING.acquire().await.map_err(|_| Error::Unavailable)?;
    let matched = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        password::matches(&password, &phc)
    })
    .await??;
    Ok(known && matched)
}

/// The second-factor check. Until TOTP lands (WP-05 T6) an operator with an enrolled secret
/// cannot sign in; T6 replaces this with the RFC 6238 check of `code`.
fn second_factor(account: &Account, code: Option<&str>) -> bool {
    let _ = code;
    !account.totp_enabled
}

/// Records the success, the token, the audit entry and the event in one transaction.
async fn succeed(
    db: &PgPool,
    bus: &Bus,
    attempt: &AttemptKey<'_>,
    account: Account,
) -> Result<SignedIn, Error> {
    let client = ClientAddr(attempt.client);
    let token = Token::from_random(random::<TOKEN_BYTES>()?);
    let expires_at = attempt.at + LOGIN_LIFETIME;
    let new_token = NewToken {
        operator: account.id,
        hash: token.hash(),
        scope: Scope::Write,
        expires_at,
    };
    let operator = Operator {
        id: account.id,
        name: account.username.clone(),
    };
    let mut tx = db.begin().await?;
    store::finish_attempt(&mut *tx, attempt.id, Outcome::Ok).await?;
    store::insert_token(&mut tx, &new_token, attempt.at).await?;
    let message = format!("Operator {} signed in.", operator.name);
    events::audit(&mut tx, attempt.at, audit_entry(operator, AuditAction::Login, client)).await?;
    events::record(&mut tx, bus, attempt.at, access_event(Severity::Info, message)).await?;
    tx.commit().await?;
    Ok(SignedIn {
        token,
        expires_at,
        account,
    })
}

/// Records a failed sign-in as an `ACCESS` warning. The typed username is named only when it
/// belongs to an operator: an unknown one may be a password typed in the wrong field.
async fn record_failure(
    db: &PgPool,
    bus: &Bus,
    attempt: &AttemptKey<'_>,
    account: Option<&Account>,
) -> Result<(), Error> {
    let who = account.map_or_else(
        || "an unknown username".to_owned(),
        |account| format!("operator {}", account.username),
    );
    let message = format!("Sign-in failed for {who} from {}.", attempt.client);
    let mut conn = db.acquire().await?;
    events::record(&mut conn, bus, attempt.at, access_event(Severity::Warning, message)).await?;
    Ok(())
}

fn audit_entry(operator: Operator, action: AuditAction, client: ClientAddr) -> NewAudit {
    NewAudit {
        target: AuditTarget::Operator(operator.id),
        operator,
        action,
        client,
        change: serde_json::json!({}),
    }
}

const fn access_event(severity: Severity, message: String) -> NewEvent {
    NewEvent {
        severity,
        source: Source::Access,
        device: None,
        message,
    }
}

/// Whether an operator could have this username (see `operator set`); others skip the lookup.
pub(super) fn possible_username(username: &str) -> bool {
    (1..=64).contains(&username.chars().count())
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The username as `login_attempts` keeps it: at most 128 characters, no NUL (which
/// PostgreSQL text refuses).
fn stored_username(username: &str) -> String {
    username
        .chars()
        .take(MAX_STORED_USERNAME)
        .map(|c| if c == '\0' { char::REPLACEMENT_CHARACTER } else { c })
        .collect()
}
