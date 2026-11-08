//! What a share leaves behind (S-6, K4 §10, LOG-005): the audit entries `share.create` and
//! `share.end`, each with an Info event of fixed text, written together in one transaction.
//! None holds the token (LOG-007).

use chrono::{DateTime, Utc};
use serde_json::json;

use super::{
    Error, Sharing,
    model::{SessionShare, ShareEnd},
};
use crate::{
    events::{self, AuditAction, AuditTarget, ClientAddr, Operator, Severity, Source, TrailEntry},
    ids::{DeviceId, ShareId},
    sessions::Actor,
};

/// The event of `share.create`.
const CREATED: &str = "An operator shared a session.";

/// A share's end, as `share.end` records it.
pub(super) struct Ended {
    pub(super) id: ShareId,
    pub(super) owner: Operator,
    /// Where the end was asked from: `endShare`'s caller, else where the owner shared from.
    pub(super) client: ClientAddr,
    pub(super) device: Option<(DeviceId, String)>,
    pub(super) end: ShareEnd,
}

/// Writes `entry` by `actor` from `client` at `now`, in one transaction.
async fn write(
    sharing: &Sharing,
    actor: (Operator, ClientAddr),
    now: DateTime<Utc>,
    entry: TrailEntry,
) -> Result<(), Error> {
    let trail = events::Trail::new(&sharing.bus, actor, now);
    let mut tx = sharing.db.begin().await?;
    trail.write(&mut tx, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// The audit entry `share.create`, by `by` at `by.at`, with the session, access, expiry and
/// most viewers; and its event about `device`.
///
/// # Errors
/// `Database` or `Events` when either cannot be written; then neither is.
pub(super) async fn created(
    sharing: &Sharing,
    by: &Actor,
    device: Option<(DeviceId, String)>,
    share: &SessionShare,
) -> Result<(), Error> {
    let entry = TrailEntry {
        source: Source::Session,
        severity: Severity::Info,
        device,
        message: CREATED.to_owned(),
        action: AuditAction::Create,
        target: AuditTarget::Share(share.id),
        change: json!({
            "sessionId": share.session.to_string(),
            "access": share.access.as_str(),
            "expiresAt": share.expires_at,
            "maxViewers": share.max_viewers,
        }),
    };
    write(sharing, (by.operator.clone(), by.client), by.at, entry).await
}

/// The audit entry `share.end` with its cause, by the share's owner, stamped by the sharing
/// clock; and its event.
///
/// # Errors
/// `Database` or `Events` when either cannot be written; then neither is.
pub(super) async fn ended(sharing: &Sharing, ended: Ended) -> Result<(), Error> {
    let entry = TrailEntry {
        source: Source::Session,
        severity: Severity::Info,
        device: ended.device,
        message: ended.end.sentence().to_owned(),
        action: AuditAction::End,
        target: AuditTarget::Share(ended.id),
        change: json!({ "cause": ended.end.as_str() }),
    };
    let now = sharing.clock.now();
    write(sharing, (ended.owner, ended.client), now, entry).await
}
