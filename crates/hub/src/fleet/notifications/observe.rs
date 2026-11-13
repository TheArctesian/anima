//! Deciding and queueing in the heartbeat's transaction, and the reads and writes around a
//! check's channels (D-16, NOT-003). Sends nothing.

use std::collections::HashMap;

use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{
    Error,
    decision::{Notice, decide},
    model::{CheckNotifications, NewOutbox, Observation, OutboxId, status_text},
    store,
};
use crate::{ids::CheckId, text::clean};

const NAME_CHARS: usize = 200;
const MESSAGE_CHARS: usize = 500;

/// Decides what `o` calls for, stores the check's new state and queues each notification for
/// each channel the check notifies, in the caller's transaction. The state row stays locked
/// until that transaction ends.
///
/// # Errors
/// `Database` when a query fails; `Unreadable` when a stored row is not known to this version.
pub async fn observe(conn: &mut PgConnection, o: &Observation) -> Result<(), Error> {
    let before = store::lock_check_state(conn, o.check).await?;
    let resend = store::check_settings(&mut *conn, &[o.check])
        .await?
        .get(&o.check)
        .map_or(0, |settings| settings.resend_every_beats);
    let days = if o.certificate_not_after.is_some() || o.domain_not_after.is_some() {
        store::installation_settings(&mut *conn)
            .await?
            .certificate_days
    } else {
        Vec::new()
    };
    let decision = decide(o, &before, resend, &days);
    if decision.state != before {
        store::write_check_state(conn, o.check, &decision.state).await?;
    }
    if decision.notices.is_empty() {
        return Ok(());
    }
    let channels = store::check_channels(&mut *conn, &[o.check])
        .await?
        .remove(&o.check)
        .unwrap_or_default();
    for notice in &decision.notices {
        let payload = payload(o, *notice);
        for channel in &channels {
            let row = NewOutbox {
                id: OutboxId::from(Uuid::now_v7()),
                channel: *channel,
                check: Some(o.check),
                kind: notice.kind,
                payload: payload.clone(),
            };
            store::insert_outbox(conn, &row, o.at).await?;
        }
    }
    Ok(())
}

/// The placeholder values of D-21, cleaned (Q2-11). Never a secret.
fn payload(o: &Observation, notice: Notice) -> Value {
    json!({
        "device": clean(&o.device.1, NAME_CHARS),
        "check": clean(&o.check_name, NAME_CHARS),
        "status": status_text(o.status),
        "previous": status_text(o.previous),
        "message": o.message.as_deref().map(|m| clean(m, MESSAGE_CHARS)),
        "at": o.at.to_rfc3339(),
        "days": notice.days,
    })
}

/// Turns on the channels marked "on by default" for a new check, in `createCheck`'s
/// transaction (NOT-003).
///
/// # Errors
/// `Database` when a query fails; `UnknownCheck` when `check` does not exist.
pub async fn adopt_defaults(conn: &mut PgConnection, check: CheckId) -> Result<(), Error> {
    let defaults = store::on_by_default(&mut *conn).await?;
    store::set_check_channels(conn, check, &defaults).await
}

/// The channels and resend setting of each of `checks`, in three queries whatever their number
/// (API-005). A check with no settings row resends never. The channels carry no secrets.
///
/// # Errors
/// `Database` when a query fails; `Unreadable` when a stored row is not known to this version.
pub async fn for_checks(
    db: &PgPool,
    checks: &[CheckId],
) -> Result<HashMap<CheckId, CheckNotifications>, Error> {
    let links = store::check_channels(db, checks).await?;
    let settings = store::check_settings(db, checks).await?;
    let mut ids: Vec<_> = links.values().flatten().copied().collect();
    ids.sort_by_key(|id| Uuid::from(*id));
    ids.dedup();
    let stored: HashMap<_, _> = store::list_channels(db, Some(&ids))
        .await?
        .into_iter()
        .map(|mut channel| {
            channel.fields.secrets = None;
            (channel.id, channel)
        })
        .collect();
    Ok(checks
        .iter()
        .map(|check| {
            let channels = links
                .get(check)
                .into_iter()
                .flatten()
                .filter_map(|id| stored.get(id).cloned())
                .collect();
            let resend_every_beats = settings
                .get(check)
                .map_or(0, |settings| settings.resend_every_beats);
            (
                *check,
                CheckNotifications {
                    channels,
                    resend_every_beats,
                },
            )
        })
        .collect())
}
