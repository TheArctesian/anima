//! The sign-in of a device of a hosted hub (hosted-mode.md HMD-037, R-129). The Anima.ing account
//! is the only way in: the link domain checks the device and its claim, then this file gives the
//! device a login token of the hub's one operator. The operator has a password that no one
//! knows, and a hosted hub refuses every password sign-in (`Error::Hosted`).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{
    ClientAddr, Error, Operator,
    actor::{self, Declaration, Signals},
    graphql::AuthPayload,
    hash_password,
    login::{SignedIn, random},
    password::Password,
    store::{self, NewToken},
    tokens::{LOGIN_LIFETIME, Scope, TOKEN_BYTES, Token, TokenKind},
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
    issue(db, bus, device).await.map_err(Error::into_graphql)
}

async fn issue(db: &PgPool, bus: &Bus, device: HostedDevice<'_>) -> Result<AuthPayload, Error> {
    let HostedDevice {
        client,
        name: device_name,
        at: now,
    } = device;
    // A password of 256 random bits, kept by no one: the operator row needs a hash.
    let unknown = Password::new(URL_SAFE_NO_PAD.encode(random::<32>()?));
    let password_hash = hash_password(unknown).await?;
    let token = Token::from_random(random::<TOKEN_BYTES>()?);
    let expires_at = now + LOGIN_LIFETIME;
    let mut tx = db.begin().await?;
    let account = store::hosted_operator(&mut tx, &password_hash, now).await?;
    let new_token = NewToken {
        operator: account.id,
        hash: token.hash(),
        kind: TokenKind::Login,
        scope: Scope::Write,
        expires_at,
    };
    store::insert_token(&mut tx, &new_token, now).await?;
    let signals = Signals {
        token: TokenKind::Login,
        made_for: None,
        declared: Declaration::default(),
    };
    let operator = Operator {
        id: account.id,
        name: account.username.clone(),
        actor: actor::actor(signals, None),
    };
    let kind = operator.actor.kind();
    let audit = NewAudit {
        target: AuditTarget::Operator(operator.id),
        operator,
        action: AuditAction::Login,
        client,
        change: serde_json::json!({ "device": device_name, "through": "account" }),
    };
    events::audit(&mut tx, now, audit).await?;
    let message = format!("The device {device_name} signed in with the Anima.ing account.");
    let event = NewEvent {
        severity: Severity::Info,
        source: Source::Access,
        device: None,
        message,
        actor: kind,
    };
    events::record(&mut tx, bus, now, event).await?;
    tx.commit().await?;
    Ok(AuthPayload(SignedIn {
        token,
        expires_at,
        account,
    }))
}
