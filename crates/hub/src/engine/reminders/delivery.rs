//! The sender of the firings (K-E6 Section 5; REM-005, REM-008, REM-009, REM-010): the check at
//! send time, the words for a lock screen, and the outbox of `notifications`. The engine builds
//! no second transport. The task of each minute runs the sweep and then the sender.

use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;

use super::{
    Error,
    live::Live,
    model::Class,
    sending::{Sendable, claim_send, waiting},
    store,
    sweep::{Reading, sweep},
};
use crate::{
    clock::Clock,
    fleet::{
        credentials::Vault,
        events::{self, ActorKind, Bus, NewEvent, Severity, Source},
        notifications::{self, Reminder},
    },
};

/// How often the sweep and the sender run (REM-002).
const MINUTE: Duration = Duration::from_secs(60);
/// The most firings one pass sends; the next pass sends on.
const BATCH: i64 = 100;
/// The reason of a firing whose channels are ntfy channels with a topic that is not random.
pub const TOPIC_NOT_RANDOM: &str = "topic_not_random";
/// The reason of a firing whose rule has no channel and no channel is on by default.
pub const NO_CHANNEL: &str = "no_channel";

/// The title and the body that leave the hub (REM-009). For Class C: the name of the rule and
/// the text of the firing. For Class A or B: fixed words, so that no name, title or value of
/// the operator's private records is on a lock screen or at a notification server.
#[must_use]
pub fn lock_screen(class: Class, rule_name: &str, text: &str) -> (String, String) {
    match class {
        Class::C => (rule_name.to_owned(), text.to_owned()),
        Class::A | Class::B => ("Anima".to_owned(), "You have a reminder.".to_owned()),
    }
}

/// What the sender needs.
#[derive(Clone)]
pub struct Sending {
    pub db: PgPool,
    pub bus: Bus,
    pub vault: Vault,
    /// The open applications: the same value as the schema has.
    pub live: Live,
}

/// The event of a firing that was queued: the name of the rule for Class C, no name else.
fn fired(sendable: &Sendable) -> NewEvent {
    let message = match sendable.class {
        Class::C => format!("The reminder {} was sent.", sendable.rule_name),
        Class::A | Class::B => "A reminder was sent.".to_owned(),
    };
    NewEvent {
        severity: Severity::Info,
        source: Source::Hub,
        device: None,
        message,
        actor: ActorKind::Hub,
    }
}

/// Sends one waiting firing in one transaction: the check at send time, then the firing to the
/// open applications or, when none is open, one outbox row for each channel. A firing that no
/// application and no channel takes becomes `suppressed` with its reason. `true` when it was
/// sent.
async fn send_one(
    sending: &Sending,
    firing: crate::ids::FiringId,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut tx = sending.db.begin().await?;
    let Some(sendable) = claim_send(&mut tx, firing, now).await? else {
        tx.commit().await?;
        return Ok(false);
    };
    if sending.live.open() > 0 {
        let shown = store::firing(&mut tx, firing).await?;
        events::record(&mut tx, &sending.bus, now, fired(&sendable)).await?;
        tx.commit().await?;
        // After the commit: an application that answers at once finds the firing `sent`.
        if let Some(shown) = shown {
            sending.live.publish(shown);
        }
        return Ok(true);
    }
    let (title, body) = lock_screen(sendable.class, &sendable.rule_name, &sendable.text);
    let reminder = Reminder {
        firing: firing.into(),
        title,
        body,
    };
    let queued = notifications::queue_reminder(
        &mut tx,
        &sending.vault,
        &sendable.channels,
        (&reminder, now),
    )
    .await?;
    if queued.rows == 0 {
        let reason = if queued.topic_not_random > 0 {
            TOPIC_NOT_RANDOM
        } else {
            NO_CHANNEL
        };
        store::decide(&mut tx, firing, ("suppressed", Some(reason)), now).await?;
    } else {
        events::record(&mut tx, &sending.bus, now, fired(&sendable)).await?;
    }
    tx.commit().await?;
    Ok(queued.rows > 0)
}

/// Sends each firing that waits at `now`, oldest first, at most 100, and gives how many were
/// sent: to the open applications or, when none is open, to a channel. One firing that fails does not stop the others: the first error is the
/// result after all were tried.
///
/// # Errors
/// `Database`, `Notifications`, `Events`.
pub async fn send_waiting(sending: &Sending, now: DateTime<Utc>) -> Result<u64, Error> {
    let mut conn = sending.db.acquire().await?;
    let firings = waiting(&mut conn, now, BATCH).await?;
    drop(conn);
    let (mut sent, mut failed) = (0, None);
    for firing in firings {
        match send_one(sending, firing, now).await {
            Ok(was_sent) => sent += u64::from(was_sent),
            Err(error) => failed = failed.or(Some(error)),
        }
    }
    failed.map_or(Ok(sent), Err)
}

/// Runs the sweep and then the sender each minute until the handle is aborted. A pass that
/// fails is logged by its fixed words, which hold no text of a rule, and the next pass tries
/// again.
#[must_use]
pub fn spawn_minute(
    sending: Sending,
    reading: Reading,
    clock: Arc<dyn Clock>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(MINUTE);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let now = clock.now();
            if let Err(error) = sweep(&sending.db, &reading, now).await {
                tracing::warn!(%error, "the reminder sweep failed");
            }
            if let Err(error) = send_waiting(&sending, now).await {
                tracing::warn!(%error, "a reminder was not sent");
            }
        }
    })
}
