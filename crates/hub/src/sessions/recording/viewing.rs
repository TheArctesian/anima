//! `openRecording` (K1, K4 §12, RA-041): a single-use playback ticket for a recording, with
//! its audit entry `recording.view`.

use serde_json::json;
use sqlx::PgPool;

use super::{Error, model::Catalogued, store, tickets::PlaybackTicket};
use crate::{
    events::{self, AuditAction, AuditTarget, NewAudit},
    ids::RecordingId,
    sessions::{
        Actor, Live, Ticket,
        tickets::{LIFETIME, TOKEN_BYTES, TicketToken},
    },
};

/// Issues a ticket that opens `/recording` once, from `by.client`, within 30 s of `by.at`, and
/// writes `recording.view` in the same transaction: the ticket is held only when the entry is
/// committed. A recording still being written may be opened; it plays up to its last written
/// part (K4 §12).
///
/// # Errors
/// [`Error::NotFound`] for an unknown id; [`Error::ShuttingDown`] or [`Error::Busy`] when no
/// ticket can be issued now; `Random`, `Events`, `Database` or `StoredValue` when a
/// dependency fails, and then no ticket is held.
pub(super) async fn open_recording(
    db: &PgPool,
    live: &Live,
    by: &Actor,
    id: RecordingId,
) -> Result<Ticket, Error> {
    if live.is_shutting_down() {
        return Err(Error::ShuttingDown);
    }
    let mut tx = db.begin().await?;
    let recording = store::catalogued(&mut *tx, id)
        .await?
        .ok_or(Error::NotFound(id))?;
    let token = random_token()?;
    let hash = token.hash();
    let expires_at = by.at + LIFETIME;
    events::audit(&mut tx, by.at, view(by, &recording))
        .await
        .map_err(Error::Events)?;
    let ticket = PlaybackTicket {
        operator: by.operator.clone(),
        client: by.client,
        recording: id,
        session: recording.session,
        expires_at,
    };
    live.playback()
        .issue(hash, ticket, by.at)
        .map_err(|_| Error::Busy)?;
    if let Err(error) = tx.commit().await {
        live.playback().withdraw(hash);
        return Err(error.into());
    }
    Ok(Ticket::playback(
        recording.session,
        recording.protocol,
        token,
        expires_at,
    ))
}

/// The audit entry `recording.view`: who opened which recording, never a path or content.
fn view(by: &Actor, recording: &Catalogued) -> NewAudit {
    NewAudit {
        operator: by.operator.clone(),
        action: AuditAction::View,
        target: AuditTarget::Recording(recording.id),
        client: by.client,
        change: json!({
            "sessionId": recording.session.to_string(),
            "deviceName": recording.device_name,
            "protocol": recording.protocol.as_str(),
        }),
    }
}

/// Random bytes from the operating system.
fn random_token() -> Result<TicketToken, Error> {
    let mut bytes = [0; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(TicketToken::from_random(bytes))
}
