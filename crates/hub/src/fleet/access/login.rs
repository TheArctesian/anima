//! Signing in: the throttle, the password check, the token and their records (SEC-002,
//! LOG-005, D-3, D-7, D-8). Signing out is in `sign_out`.

mod sign_out;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::{
    ClientAddr, Error, Identity, Operator,
    actor::{self, Declaration, Signals},
    password::{self, DUMMY_HASH, Password},
    store::{self, Account, AttemptKey, NewToken, Outcome},
    throttle,
    tokens::{BROWSER_LIFETIME, LOGIN_LIFETIME, Scope, TOKEN_BYTES, Token, TokenKind},
    totp::{self, SecondFactor},
};
pub(super) use sign_out::logout;
use sign_out::retire;

use crate::fleet::{
    credentials::Vault,
    events::{
        self, Actor, ActorKind, AuditAction, AuditTarget, Bus, NewAudit, NewEvent, Severity, Source,
    },
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
    /// Whether the request declared an AI agent (LOG-016).
    pub declared: Declaration,
}

impl Attempt {
    /// Who the attempt's records name as the actor: the kind of token it asks for, raised to
    /// Agent by the declaration (LOG-014, LOG-018).
    fn actor(&self) -> Actor {
        let signals = Signals {
            token: self.issue.kind,
            made_for: None,
            declared: self.declared,
        };
        actor::actor(signals, None)
    }
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
            .and_then(|identity| Some((identity.token(), identity.operator()?.clone())))
            .map(|(token, operator)| Replaced { token, operator });
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
    let actor = attempt.actor();
    let kind = actor.kind();
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
            actor,
        };
        if let Some(signed_in) = succeed(db, bus, &key, checked).await? {
            return Ok(signed_in);
        }
    }
    let failed = failed_message(&key, name.as_deref());
    record_access(db, bus, &key, access_event(Severity::Warning, failed, kind)).await?;
    match factor {
        SecondFactor::Unopenable(error) => {
            let unopenable = unopenable_message(name.as_deref());
            record_access(
                db,
                bus,
                &key,
                access_event(Severity::Error, unopenable, kind),
            )
            .await?;
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
    let failures = store::failures(&mut *conn, key.client, key.at, key.id).await?;
    if throttle::refused(&failures, key.at) {
        store::finish_attempt(&mut *conn, key.id, Outcome::Refused).await?;
        return Err(Error::WrongCredentials);
    }
    if !possible_username(username) {
        return Ok(None);
    }
    Ok(store::account_by_username(&mut *conn, username).await?)
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

/// An account whose password and second factor were checked, the TOTP step to claim, the
/// token to give and who asked for it.
struct Checked {
    account: Account,
    step: Option<i64>,
    issue: Issue,
    actor: Actor,
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
        actor,
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
        actor,
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
    if let Some(mut replaced) = issue.replaces {
        // This sign-in deletes the old token, so its sign-out is this request's (LOG-014).
        replaced.operator.actor = operator.actor.clone();
        retire(&mut tx, replaced, client, attempt.at).await?;
    }
    let message = format!("Operator {} signed in.", operator.name);
    let kind = operator.actor.kind();
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
        access_event(Severity::Info, message, kind),
    )
    .await?;
    tx.commit().await?;
    Ok(Some(SignedIn {
        token,
        expires_at,
        account,
    }))
}

/// The `ACCESS` warning of a failed sign-in. The typed username is named only when it
/// belongs to an operator: an unknown one may be a password typed in the wrong field.
fn failed_message(attempt: &AttemptKey, operator: Option<&str>) -> String {
    let who = operator.map_or_else(
        || "an unknown username".to_owned(),
        |name| format!("operator {name}"),
    );
    format!("Sign-in failed for {who} from {}.", attempt.client)
}

/// The `ACCESS` error that the operator's TOTP secret does not open with the hub key, so the
/// operator cannot sign in until it is enrolled again. Names no secret.
fn unopenable_message(operator: Option<&str>) -> String {
    let name = operator.unwrap_or_default();
    format!(
        "The second factor of operator {name} cannot be checked: its TOTP secret does not \
         open with the hub key. Enrol it again with anima-hub operator totp."
    )
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

const fn access_event(severity: Severity, message: String, actor: ActorKind) -> NewEvent {
    NewEvent {
        severity,
        source: Source::Access,
        device: None,
        message,
        actor,
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
