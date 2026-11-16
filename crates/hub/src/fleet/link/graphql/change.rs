//! One link change with its audit entry and event, run to the end in a task of its own: a
//! client that goes away while the link works still leaves the audit entry of the change the
//! link made (LOG-005). The value comes back only to the resolver that asked for it.
//!
//! The entries name the device, never the invite and never the device's key (LOG-007).

use std::{future::Future, sync::Arc};

use async_graphql::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::json;
use sqlx::PgPool;

use super::{
    super::model::ListedInvite, Control, Error, ListedDevice, MadeInvite, Request, Revoked,
    to_graphql,
};
use crate::{
    api_errors::{internal, internal_stating, pool, schema_dependency},
    clock::Clock,
    fleet::events::{
        self, AuditAction, AuditTarget, Bus, ClientAddr, Operator, Severity, Source, Trail,
        TrailEntry,
    },
    text::clean,
};

/// One change of the link: the call, the input field its refusal names, the audit entry of
/// its value, and what to do with the value when that entry cannot be written (the words the
/// operator then reads).
pub(in crate::fleet::link) struct Change<C, E, U> {
    pub call: C,
    pub field: Option<&'static str>,
    pub entry_of: E,
    pub unwritten: U,
}

/// Runs the call, then writes its entry in its own transaction, both in a spawned task, so
/// dropping the request stops neither. A failed call is the K7 error on `field`. When the
/// entry cannot be written, `unwritten` gets the value (an invite is then withdrawn, not
/// shown) and the answer is `INTERNAL` with its words: the link acted (LOG-005).
pub(super) async fn run<T, C, E, U, F>(ctx: &Context<'_>, change: Change<C, E, U>) -> Result<T>
where
    T: Send + 'static,
    C: Future<Output = Result<T, Error>> + Send + 'static,
    E: FnOnce(&T) -> TrailEntry + Send + 'static,
    U: FnOnce(T) -> F + Send + 'static,
    F: Future<Output = &'static str> + Send,
{
    let pool = pool(ctx)?.clone();
    let bus = schema_dependency::<Bus>(ctx)?.clone();
    let who = (
        schema_dependency::<Operator>(ctx)?.clone(),
        *schema_dependency::<ClientAddr>(ctx)?,
    );
    let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
    let Change {
        call,
        field,
        entry_of,
        unwritten,
    } = change;
    let task = tokio::spawn(async move {
        let value = call.await?;
        let entry = entry_of(&value);
        Ok::<_, Error>(
            match write(&pool, Trail::new(&bus, who, now), entry).await {
                Ok(()) => Ok(value),
                Err(cause) => Err((unwritten(value).await, cause)),
            },
        )
    });
    match task
        .await
        .map_err(internal)?
        .map_err(|error| to_graphql(&error, field))?
    {
        Ok(value) => Ok(value),
        Err((words, cause)) => Err(internal_stating(words, cause)),
    }
}

/// Why the audit entry of a change the link made was not written.
#[derive(Debug, thiserror::Error)]
enum Unwritten {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Trail(#[from] events::Error),
}

async fn write(pool: &PgPool, trail: Trail<'_>, entry: TrailEntry) -> Result<(), Unwritten> {
    let mut tx = pool.begin().await?;
    trail.write(&mut tx, entry).await?;
    Ok(tx.commit().await?)
}

/// What the operator reads when a revocation has no audit entry.
pub(super) const REVOKED_UNAUDITED: &str =
    "The link revoked the device, but the hub could not write its audit entry.";
const INVITE_CANCELLED: &str = "The link made the invite, but the hub could not write its \
                                audit entry, so it cancelled the invite. Make a new one.";
const INVITE_UNAUDITED: &str = "The link made the invite, but the hub could not write its \
                                audit entry. The invite is not shown; it expires in 15 minutes.";

/// Cancels the open invite for `name` that expires at `expires`, the one the link just made;
/// the words say whether it is cancelled. `invites.make` gives no id, so the hub finds it in
/// `invites.list`, which holds no invite text.
pub(super) async fn withdrawn(
    control: &Control,
    name: &str,
    expires: DateTime<Utc>,
) -> &'static str {
    let open: Result<Vec<ListedInvite>, _> = control.call(Request::InvitesList).await;
    let made = (open.ok().into_iter().flatten())
        .find(|invite| invite.name == name && invite.expires == expires);
    let Some(made) = made else {
        return INVITE_UNAUDITED;
    };
    let cancel = Request::InvitesCancel { id: &made.id };
    match control.call::<()>(cancel).await {
        Ok(()) => INVITE_CANCELLED,
        Err(_) => INVITE_UNAUDITED,
    }
}

/// An invite for the device `name` was made (LNK-003).
pub(super) fn invite_made(name: &str, made: &MadeInvite) -> TrailEntry {
    let name = clean(name, super::NAME_CHARS);
    let expires_at = made.expires.to_rfc3339_opts(SecondsFormat::Secs, true);
    entry(
        AuditAction::Create,
        format!("Made a link invite for the device {name}."),
        json!({ "device": name, "expiresAt": expires_at }),
    )
}

/// The device left the link (LNK-004).
pub(super) fn device_revoked(device: &ListedDevice, revoked: &Revoked) -> TrailEntry {
    let name = clean(&device.name, super::NAME_CHARS);
    let fingerprint = clean(&device.fingerprint, super::NAME_CHARS);
    entry(
        AuditAction::Delete,
        format!("Revoked the device {name} ({fingerprint}) from the link."),
        json!({
            "device": name,
            "fingerprint": fingerprint,
            "streamsClosed": revoked.streams_closed,
        }),
    )
}

fn entry(action: AuditAction, message: String, change: serde_json::Value) -> TrailEntry {
    TrailEntry {
        source: Source::Access,
        severity: Severity::Info,
        device: None,
        message,
        action,
        target: AuditTarget::Link,
        change,
    }
}
