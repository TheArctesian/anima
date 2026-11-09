//! What happens to one claimed outbox row (D-16, NOT-001, NOT-002).
//!
//! The row's channel is read without a lock and the send happens outside any transaction, so
//! no database lock is held while a target answers. Then one transaction writes the delivery,
//! finishes or reschedules the row and, when the row is given up, writes one Warning event.
//!
//! - A channel deleted since the row was queued takes the row with it (cascade); a row whose
//!   channel is gone by the time it is read is ended unsent.
//! - A channel the operator turned off for the row's check since it was queued ends the row
//!   unsent, with no delivery and no event. Channels have no on/off switch of their own (K1,
//!   K2): unlinking is how a channel is turned off. A row whose check is gone is still sent, as
//!   K2 says (it was decided while the check existed).
//! - A channel whose settings or secrets cannot be read sends nothing and gets a delivery with
//!   the reason, then the row is given up with a Warning event: `notificationChannels` hides
//!   such a channel, so the delivery and the event are how the operator learns of it.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
    Dispatcher, Fault,
    failure::{Failure, delivery},
    retry::{self, Next},
};
use crate::{
    events::{self, NewEvent, Severity, Source},
    ids::DeliveryId,
    notifications::{
        Delivery, InstallationSettings, NotificationKind, Outbox, store, template::Values,
    },
};

/// What became of a claimed row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Finished {
    Sent,
    /// Sent nothing and wrote nothing: the channel is gone or turned off for the check.
    Ended,
    /// The send failed; the row is rescheduled or given up.
    Failed(Failure),
}

impl Dispatcher {
    /// Sends `row` at `now` as worded by its channel or `defaults`, and records the outcome.
    ///
    /// # Errors
    /// A database or event write failed; the row stays claimed and comes back when its lease
    /// ends.
    pub(super) async fn finish(
        &self,
        row: &Outbox,
        defaults: &InstallationSettings,
        now: DateTime<Utc>,
    ) -> Result<Finished, Fault> {
        let Some(channel) = store::list_channels(&self.db, Some(&[row.channel]))
            .await?
            .pop()
        else {
            store::give_up(&self.db, row.id, now).await?;
            return Ok(Finished::Ended);
        };
        if let Some(check) = row.check
            && store::channel_turned_off(&self.db, check, channel.id).await?
        {
            store::give_up(&self.db, row.id, now).await?;
            return Ok(Finished::Ended);
        }
        let values = Values::from_payload(&row.payload);
        let sent = self
            .sender
            .send(&channel, (row.kind, &values), defaults)
            .await;
        let id = DeliveryId::from(Uuid::now_v7());
        let delivery = delivery(id, &channel, (row.kind, now), sent);
        self.record(row, &delivery, sent).await?;
        Ok(sent.map_or_else(Finished::Failed, |()| Finished::Sent))
    }

    /// Writes `delivery`, the outcome of sending `row`, and finishes, reschedules or gives up
    /// the row, in one transaction, at the delivery's time. A row another claimer finished
    /// meanwhile (its lease ended during this send) is left as it is; the delivery still
    /// records this send.
    async fn record(
        &self,
        row: &Outbox,
        delivery: &Delivery,
        sent: Result<(), Failure>,
    ) -> Result<(), Fault> {
        let now = delivery.at;
        let mut tx = self.db.begin().await.map_err(store_fault)?;
        store::insert_delivery(&mut tx, delivery).await?;
        match sent.map_err(|failure| (failure, retry::after_failure(failure, row.attempts, now))) {
            Ok(()) => {
                store::mark_sent(&mut *tx, row.id, now).await?;
            }
            Err((_, Next::RetryAt(at))) => {
                store::reschedule(&mut *tx, row.id, at).await?;
            }
            Err((failure, Next::GiveUp)) => {
                if store::give_up(&mut *tx, row.id, now).await? {
                    let warning = given_up(&delivery.channel_name, row, failure);
                    events::record(&mut tx, &self.bus, now, warning).await?;
                }
            }
        }
        tx.commit().await.map_err(store_fault)?;
        Ok(())
    }
}

fn store_fault(error: sqlx::Error) -> Fault {
    Fault::Store(error.into())
}

/// The kind as the operator reads it in an event.
const fn kind_words(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::Down => "Down",
        NotificationKind::Up => "Up",
        NotificationKind::StillDown => "Still down",
        NotificationKind::CertificateExpiry => "certificate expiry",
        NotificationKind::DomainExpiry => "domain expiry",
        NotificationKind::Test => "test",
    }
}

/// The Warning event of a row given up: the channel's name, the kind, the check and device it was
/// about, the tries and the last reason. Never a target, URL, topic, header or reply
/// (NOT-002, LOG-007).
pub(super) fn given_up(channel_name: &str, row: &Outbox, failure: Failure) -> NewEvent {
    let values = Values::from_payload(&row.payload);
    let about = match (values.check, values.device) {
        (Some(check), Some(device)) => format!(" for {check} on {device}"),
        _ => String::new(),
    };
    let tries = row.attempts.saturating_add(1);
    let code = failure
        .code
        .map(|code| format!(" {code}"))
        .unwrap_or_default();
    NewEvent {
        severity: Severity::Warning,
        source: Source::Notifications,
        device: None,
        message: format!(
            "Notification channel {} gave up a {} notification{about} after {tries} {}: {}{code}.",
            channel_name,
            kind_words(row.kind),
            if tries == 1 { "try" } else { "tries" },
            failure.reason.as_str(),
        ),
    }
}
