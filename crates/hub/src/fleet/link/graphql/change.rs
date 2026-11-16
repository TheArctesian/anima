//! One link change with its audit entry and event, run to the end in a task of its own: a
//! client that goes away while the link works still leaves the audit entry of the change the
//! link made (LOG-005). The value comes back only to the resolver that asked for it.
//!
//! The entries name the device, never the invite and never the device's key (LOG-007).

use std::{future::Future, sync::Arc};

use async_graphql::{Context, Result};
use chrono::SecondsFormat;
use serde_json::json;
use sqlx::PgPool;

use super::{Error, ListedDevice, MadeInvite, Revoked, to_graphql};
use crate::{
    api_errors::{commit, internal, pool, schema_dependency},
    clock::Clock,
    fleet::events::{
        AuditAction, AuditTarget, Bus, ClientAddr, Operator, Severity, Source, Trail, TrailEntry,
    },
    text::clean,
};

/// Runs `call`, then writes `entry_of(&value)` in its own transaction, both in a spawned task,
/// so dropping the request stops neither. A failed call is the K7 error on `field`; a failed
/// write is `INTERNAL`, and the value (an invite) is not shown.
pub(super) async fn run<T: Send + 'static>(
    ctx: &Context<'_>,
    call: impl Future<Output = Result<T, Error>> + Send + 'static,
    field: Option<&'static str>,
    entry_of: impl FnOnce(&T) -> TrailEntry + Send + 'static,
) -> Result<T> {
    let pool = pool(ctx)?.clone();
    let bus = schema_dependency::<Bus>(ctx)?.clone();
    let who = (
        schema_dependency::<Operator>(ctx)?.clone(),
        *schema_dependency::<ClientAddr>(ctx)?,
    );
    let now = schema_dependency::<Arc<dyn Clock>>(ctx)?.now();
    let change = tokio::spawn(async move {
        let value = call.await?;
        let written = write(&pool, Trail::new(&bus, who, now), entry_of(&value)).await;
        Ok::<_, Error>((value, written))
    });
    let (value, written) = change
        .await
        .map_err(internal)?
        .map_err(|error| to_graphql(&error, field))?;
    written?;
    Ok(value)
}

async fn write(pool: &PgPool, trail: Trail<'_>, entry: TrailEntry) -> Result<()> {
    let mut tx = pool.begin().await.map_err(internal)?;
    trail.write(&mut tx, entry).await.map_err(internal)?;
    commit(tx).await
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
