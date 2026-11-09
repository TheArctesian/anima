//! Signing in and out: the throttle, the password check, the token and their records
//! (SEC-002, LOG-005, D-3, D-7, D-8).

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::{
    ClientAddr, Error, Identity, Operator,
    password::{self, DUMMY_HASH, Password},
    store::{self, Account, AttemptKey, NewToken, Outcome},
    throttle,
    tokens::{BROWSER_LIFETIME, LOGIN_LIFETIME, Scope, TOKEN_BYTES, Token, TokenKind},
    totp::{self, SecondFactor},
};
use crate::{
    credentials::Vault,
    events::{self, AuditAction, AuditTarget, Bus, NewAudit, NewEvent, Severity, Source},
};

/// Password checks that run at once. Each takes 19 MiB and a CPU for tens of milliseconds,
/// so a burst of attempts queues here instead of exhausting memory.
static HASHING: Semaphore = Semaphore::const_new(2);

/// Longest username kept in `login_attempts`; longer input is cut there.
const MAX_STORED_USERNAME: usize = 128;

/// A sign-in attempt as received. No `Debug`: it holds the password.
pub(super) struct Attempt {
    pub username: String,
    pub password: Password,
    /// The TOTP code; required when the operator has enrolled TOTP.
    pub totp_code: Option<String>,
    pub client: ClientAddr,
    pub at: DateTime<Utc>,
    /// The token a success gives.
    pub issue: Issue,
}

/// The token a sign-in gives: always `write`, of this kind, for this long; and the browser
/// token it replaces, if any.
pub(super) struct Issue {
    kind: TokenKind,
    lifetime: TimeDelta,
    replaces: Option<Replaced>,
}

impl Issue {
    /// The desktop app's `login`: a login token for 30 days (D-7). It replaces nothing: the
    /// app keeps its token in the OS key store, where the hub cannot see what it overwrites.
    pub(super) const LOGIN: Self = Self {
        kind: TokenKind::Login,
        lifetime: LOGIN_LIFETIME,
        replaces: None,
    };

    /// `signInBrowser`: a browser token for 7 days, with no refresh (D-3). It replaces the
    /// browser token of `cookie`, the identity the request's `__Host-anima` cookie gave, since
    /// the new cookie overwrites that one in the browser and nothing else holds it.
    ///
    /// Only a `browser` identity is replaced, whichever operator it names: a request carries
    /// one only when its cookie came from the public origin with no `Authorization` header
    /// (A-3, A-4), so the browser that sent it held the token and could have deleted it with
    /// `logout` instead. A bearer identity is never replaced.
    pub(super) fn browser(cookie: Option<&Identity>) -> Self {
        let replaces = cookie
            .filter(|identity| identity.kind() == TokenKind::Browser)
            .map(|identity| Replaced {
                token: identity.token(),
                operator: identity.operator().clone(),
            });
        Self {
            kind: TokenKind::Browser,
            lifetime: BROWSER_LIFETIME,
            replaces,
        }
    }
}

/// A browser token that a successful sign-in deletes, and its operator.
struct Replaced {
    token: Uuid,
    operator: Operator,
}

/// A successful sign-in: the new token, shown once, and the account.
pub(super) struct SignedIn {
    pub token: Token,
    pub expires_at: DateTime<Utc>,
    pub account: Account,
}

/// Signs the operator in and issues the `write` token `attempt.issue` names. `login` and
/// `signInBrowser` share this path, so they share the throttle, the second factor and the
/// records (D-4).
///
/// Every attempt is recorded. A wrong password, an unknown username, a wrong, reused or
/// missing TOTP code, a TOTP secret that does not open, an account changed during the check
/// and a throttled attempt all give the answer of `Error::WrongCredentials`, and all but the
/// throttled one count toward the throttle.
pub(super) async fn login(
    db: &PgPool,
    bus: &Bus,
    vault: &Vault,
    attempt: Attempt,
) -> Result<SignedIn, Error> {
    let stored_name = stored_username(&attempt.username);
    let key = AttemptKey {
        id: store::start_attempt(db, attempt.at, &stored_name, attempt.client.0).await?,
        at: attempt.at,
        client: attempt.client.0,
    };
    let account = unthrottled_account(db, &key, &attempt.username).await?;
    let verified = password_matches(attempt.password, account.as_ref()).await?;
    let factor = match &account {
        Some(account) if verified => {
            totp::second_factor(vault, account, attempt.totp_code.as_deref(), attempt.at)
        }
        _ => SecondFactor::Refuse,
    };
    let name = account.as_ref().map(|account| account.username.clone());
    if let Some(account) = account
        && factor.passes()
    {
        let checked = Checked {
            account,
            step: factor.step(),
            issue: attempt.issue,
        };
        if let Some(signed_in) = succeed(db, bus, &key, checked).await? {
            return Ok(signed_in);
        }
    }
    warn_failed_sign_in(db, bus, &key, name.as_deref()).await?;
    match factor {
        SecondFactor::Unopenable(error) => {
            record_unopenable(db, bus, &key, name.as_deref()).await?;
            Err(Error::TotpSecret(error))
        }
        _ => Err(Error::WrongCredentials),
    }
}

/// The account for `username`, or `None` when no operator could have it. Refuses with
/// `Error::WrongCredentials` when the throttle turns the attempt away (D-8).
async fn unthrottled_account(
    db: &PgPool,
    key: &AttemptKey,
    username: &str,
) -> Result<Option<Account>, Error> {
    let mut conn = db.acquire().await?;
    let failures = store::failures(&mut *conn, key.client, key.at, Some(key.id)).await?;
    if throttle::refused(&failures, key.at) {
        store::finish_attempt(&mut *conn, key.id, Outcome::Refused).await?;
        return Err(Error::WrongCredentials);
    }
    if !possible_username(username) {
        return Ok(None);
    }
    Ok(store::account_by_username(&mut *conn, username).await?)
}

/// The username a failure from outside sign-in is stored under. It fails the operator username
/// check (`^[A-Za-z0-9._-]{1,64}$`), so no operator can have it: it never matches an account and
/// `operator set` never clears it by name (D-8, D-9, CR-16). A `login` typing this text as its
/// username is stored under the same text.
const OUTSIDE_SIGN_IN: &str = "(share join)";

/// Whether the login throttle turns away an attempt from `client` at `now`, by the rule and
/// the failures `login` reads (D-8). `joinShare` asks before it checks a share secret, so a
/// share miss and a wrong password count against the same address (CR-16).
///
/// Asking and then calling [`record_failure`] is two steps, so guesses sent in parallel all
/// pass before any is recorded; `login` records its attempt first and has no such gap.
///
/// # Errors
///
/// `Error::Database` if the attempts cannot be read.
pub async fn throttled(db: &PgPool, client: ClientAddr, now: DateTime<Utc>) -> Result<bool, Error> {
    let failures = store::failures(db, client.0.to_canonical(), now, None).await?;
    Ok(throttle::refused(&failures, now))
}

/// Records a failed attempt from `client` at `now` that counts toward the login throttle like a
/// wrong password (D-8). Call it only after [`throttled`] answered `false` and the secret did
/// not match: a miss recorded during a lockout would extend it (CR-16).
///
/// # Errors
///
/// `Error::Database` if the attempt cannot be recorded.
pub async fn record_failure(
    db: &PgPool,
    client: ClientAddr,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    store::start_attempt(db, now, OUTSIDE_SIGN_IN, client.0.to_canonical()).await?;
    Ok(())
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

/// Deletes the replaced browser token and audits it as a sign-out of its operator, as
/// `logout` does. A token already gone (a `logout` that committed first) is left alone.
async fn retire(
    conn: &mut PgConnection,
    replaced: Replaced,
    client: ClientAddr,
    at: DateTime<Utc>,
) -> Result<(), Error> {
    if store::delete_token(conn, replaced.token).await? {
        let entry = audit_entry(replaced.operator, AuditAction::Logout, client);
        events::audit(conn, at, entry).await?;
    }
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

/// An account whose password and second factor were checked, the TOTP step to claim and the
/// token to give.
struct Checked {
    account: Account,
    step: Option<i64>,
    issue: Issue,
}

/// Records the success, the token, the audit entry and the event in one transaction, or
/// `None` when the account changed since it was checked. The browser token the issue replaces
/// is deleted in the same transaction, so a sign-in that does not commit deletes nothing.
///
/// The transaction locks the operator row and checks that its password hash and TOTP secret
/// are still the ones checked, so an `operator set` or `operator totp` that commits during
/// the password check cannot leave a token it should have deleted: one that commits later
/// waits for the lock and then deletes the new token. The TOTP step is claimed in the same
/// transaction, so a sign-in that fails after the claim rolls it back and no step is lost.
async fn succeed(
    db: &PgPool,
    bus: &Bus,
    attempt: &AttemptKey,
    checked: Checked,
) -> Result<Option<SignedIn>, Error> {
    let Checked {
        account,
        step,
        issue,
    } = checked;
    let client = ClientAddr(attempt.client);
    let token = Token::from_random(random::<TOKEN_BYTES>()?);
    let expires_at = attempt.at + issue.lifetime;
    let new_token = NewToken {
        operator: account.id,
        hash: token.hash(),
        kind: issue.kind,
        scope: Scope::Write,
        expires_at,
    };
    let operator = Operator {
        id: account.id,
        name: account.username.clone(),
    };
    let mut tx = db.begin().await?;
    if !store::lock_unchanged(&mut tx, &account).await? {
        return Ok(None);
    }
    if let Some(step) = step {
        let sealed = account.totp_secret_enc.as_deref().unwrap_or_default();
        if !store::claim_totp_step(&mut *tx, account.id, sealed, step).await? {
            return Ok(None);
        }
    }
    store::finish_attempt(&mut *tx, attempt.id, Outcome::Ok).await?;
    store::insert_token(&mut tx, &new_token, attempt.at).await?;
    if let Some(replaced) = issue.replaces {
        retire(&mut tx, replaced, client, attempt.at).await?;
    }
    let message = format!("Operator {} signed in.", operator.name);
    events::audit(
        &mut tx,
        attempt.at,
        audit_entry(operator, AuditAction::Login, client),
    )
    .await?;
    events::record(
        &mut tx,
        bus,
        attempt.at,
        access_event(Severity::Info, message),
    )
    .await?;
    tx.commit().await?;
    Ok(Some(SignedIn {
        token,
        expires_at,
        account,
    }))
}

/// Records a failed sign-in as an `ACCESS` warning. The typed username is named only when it
/// belongs to an operator: an unknown one may be a password typed in the wrong field.
async fn warn_failed_sign_in(
    db: &PgPool,
    bus: &Bus,
    attempt: &AttemptKey,
    operator: Option<&str>,
) -> Result<(), Error> {
    let who = operator.map_or_else(
        || "an unknown username".to_owned(),
        |name| format!("operator {name}"),
    );
    let message = format!("Sign-in failed for {who} from {}.", attempt.client);
    record_access(db, bus, attempt, access_event(Severity::Warning, message)).await
}

/// Records, as an `ACCESS` error, that the operator's TOTP secret does not open with the hub
/// key, so the operator cannot sign in until it is enrolled again. Names no secret.
async fn record_unopenable(
    db: &PgPool,
    bus: &Bus,
    attempt: &AttemptKey,
    operator: Option<&str>,
) -> Result<(), Error> {
    let name = operator.unwrap_or_default();
    let message = format!(
        "The second factor of operator {name} cannot be checked: its TOTP secret does not \
         open with the hub key. Enrol it again with anima-hub operator totp."
    );
    record_access(db, bus, attempt, access_event(Severity::Error, message)).await
}

async fn record_access(
    db: &PgPool,
    bus: &Bus,
    attempt: &AttemptKey,
    event: NewEvent,
) -> Result<(), Error> {
    let mut conn = db.acquire().await?;
    events::record(&mut conn, bus, attempt.at, event).await?;
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
pub(super) fn stored_username(username: &str) -> String {
    username
        .chars()
        .take(MAX_STORED_USERNAME)
        .map(|c| {
            if c == '\0' {
                char::REPLACEMENT_CHARACTER
            } else {
                c
            }
        })
        .collect()
}
