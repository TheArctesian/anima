//! The sign-in of a device of a hosted hub (hosted-mode.md HMD-037, R-129). The Anima.ing account
//! is the only way in: the link domain checks the device and its claim, then this file gives the
//! device a login token of the hub's one operator. The operator has a password that no one
//! knows, and a hosted hub refuses every password sign-in (`Error::Hosted`).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    super::{
        ClientAddr, Error, Operator,
        actor::{self, Declaration, Signals},
        browser,
        graphql::AuthPayload,
        hash_password,
        password::Password,
        store::{self, NewToken},
        tokens::{
            HOSTED_BROWSER_IDLE, HOSTED_BROWSER_LIFETIME, LOGIN_LIFETIME, Scope, TOKEN_BYTES,
            Token, TokenKind,
        },
    },
    SignedIn, random,
};
use crate::fleet::events::{
    self, AuditAction, AuditTarget, Bus, NewAudit, NewEvent, Severity, Source,
};

/// The device that signs in: where its request came from, its name on the link, and when.
pub struct HostedDevice<'a> {
    pub client: ClientAddr,
    pub name: &'a str,
    pub at: DateTime<Utc>,
}

/// Gives the device a login token for 30 days (D-7), with its audit entry and event
/// in one transaction. The caller has checked that the device joined the link with the invite
/// of a claim, and has used that claim.
///
/// # Errors
///
/// `INTERNAL` (K7) when the token cannot be made or kept; the process log gets the fault.
pub async fn sign_in_hosted(
    db: &PgPool,
    bus: &Bus,
    device: HostedDevice<'_>,
) -> async_graphql::Result<AuthPayload> {
    let entry = Entry {
        client: device.client,
        at: device.at,
        token: TokenKind::Login,
        change: serde_json::json!({ "device": device.name, "through": "account" }),
        message: format!(
            "The device {} signed in with the Anima.ing account.",
            device.name
        ),
    };
    issue(db, bus, entry)
        .await
        .map(AuthPayload)
        .map_err(Error::into_graphql)
}

/// Gives a browser the session of a hosted hub (HMD-061, HMD-062): a browser token for at most
/// 12 hours that ends after 30 minutes with no request, with its audit entry and event in one
/// transaction. The caller has used the one-time ticket that the Anima.ing cloud asked for.
///
/// # Errors
///
/// `INTERNAL` (K7) when the token cannot be made or kept; the process log gets the fault.
pub async fn sign_in_hosted_browser(
    db: &PgPool,
    bus: &Bus,
    client: ClientAddr,
    at: DateTime<Utc>,
) -> async_graphql::Result<BrowserSession> {
    let entry = Entry {
        client,
        at,
        token: TokenKind::Browser,
        change: serde_json::json!({ "browser": true, "through": "account" }),
        message: "A browser signed in with the Anima.ing account.".to_owned(),
    };
    let signed_in = issue(db, bus, entry).await.map_err(Error::into_graphql)?;
    Ok(BrowserSession {
        set_cookie: browser::signed_in_for(&signed_in.token, HOSTED_BROWSER_LIFETIME),
        expires_at: signed_in.expires_at,
    })
}

/// The browser session of a hosted hub, as the answer carries it: the `Set-Cookie` value, which
/// holds the token and is marked sensitive (LOG-007), and when the session ends at the latest.
pub struct BrowserSession {
    pub set_cookie: axum::http::HeaderValue,
    pub expires_at: DateTime<Utc>,
}

/// One way in by the account: who asks, when, which token it gets and what the trail says.
struct Entry {
    client: ClientAddr,
    at: DateTime<Utc>,
    token: TokenKind,
    change: serde_json::Value,
    message: String,
}

/// How long the token of a way in lasts, and its longest time with no request: a device has
/// 30 days (D-7) and no such rule; a browser has 12 hours and 30 minutes (HMD-062).
fn terms(kind: TokenKind) -> (chrono::TimeDelta, Option<chrono::TimeDelta>) {
    if kind == TokenKind::Browser {
        (HOSTED_BROWSER_LIFETIME, Some(HOSTED_BROWSER_IDLE))
    } else {
        (LOGIN_LIFETIME, None)
    }
}

async fn issue(db: &PgPool, bus: &Bus, entry: Entry) -> Result<SignedIn, Error> {
    let Entry {
        client,
        at: now,
        token: kind,
        change,
        message,
    } = entry;
    // A password of 256 random bits, kept by no one: the operator row needs a hash.
    let unknown = Password::new(URL_SAFE_NO_PAD.encode(random::<32>()?));
    let password_hash = hash_password(unknown).await?;
    let token = Token::from_random(random::<TOKEN_BYTES>()?);
    let (lifetime, idle) = terms(kind);
    let expires_at = now + lifetime;
    let mut tx = db.begin().await?;
    let account = store::hosted_operator(&mut tx, &password_hash, now).await?;
    let new_token = NewToken {
        operator: account.id,
        hash: token.hash(),
        kind,
        scope: Scope::Write,
        expires_at,
    };
    store::insert_token_with_idle(&mut tx, &new_token, now, idle).await?;
    let signals = Signals {
        token: kind,
        made_for: None,
        declared: Declaration::default(),
    };
    let operator = Operator {
        id: account.id,
        name: account.username.clone(),
        actor: actor::actor(signals, None),
    };
    let actor = operator.actor.kind();
    let audit = NewAudit {
        target: AuditTarget::Operator(operator.id),
        operator,
        action: AuditAction::Login,
        client,
        change,
    };
    events::audit(&mut tx, now, audit).await?;
    let event = NewEvent {
        severity: Severity::Info,
        source: Source::Access,
        device: None,
        message,
        actor,
    };
    events::record(&mut tx, bus, now, event).await?;
    tx.commit().await?;
    Ok(SignedIn {
        token,
        expires_at,
        account,
    })
}
