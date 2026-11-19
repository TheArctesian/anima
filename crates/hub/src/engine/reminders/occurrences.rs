//! The occurrences that the engine gives with no extension, and the interface of an extension
//! that gives its own (K-E6 Section 1; ENG-001). Pure, apart from the trait that an extension
//! implements.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::model::{Class, Occurrence};
use crate::{
    engine::records::Reference,
    fleet::{
        credentials::operational::{OperationalKey, VersionState},
        events::EventSummary,
    },
};

/// The first words of the event of a firing. An event that starts with them gives no
/// occurrence, so that a rule about the events of the hub does not fire on its own reminders.
pub(super) const OWN_EVENTS: [&str; 2] = ["The reminder ", "A reminder was sent."];
/// The subject of an operational key that ends at a time.
const KEY_EXPIRY: &str = "keys.expiry";
/// The subject of an operational key whose rotation is due at a time.
const KEY_ROTATION: &str = "keys.rotation";
const TITLE_MAX: usize = 200;

/// An extension could not give its occurrences now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the {0} extension could not give its occurrences")]
pub struct Unavailable(pub &'static str);

/// What an extension gives on the sweep, for what has no moment of storage. It is given to the
/// engine at wiring; an extension that is off is not read.
#[async_trait]
pub trait Triggers: Send + Sync {
    /// The key of the extension (K-E7).
    fn name(&self) -> &'static str;

    /// The things that end at a time.
    async fn expiries(
        &self,
        db: &PgPool,
        now: DateTime<Utc>,
    ) -> Result<Vec<Occurrence>, Unavailable>;

    /// The numbers at a time.
    async fn measures(
        &self,
        db: &PgPool,
        now: DateTime<Utc>,
    ) -> Result<Vec<Occurrence>, Unavailable>;
}

/// Every [`Triggers`] of the hub, built once in wiring.
pub type Extensions = Vec<Arc<dyn Triggers>>;

fn one_line(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(TITLE_MAX)
        .collect()
}

/// The occurrence of one event of the event log: a `Record` with the source of the event, its
/// severity as the kind, and its sentence as the title (Class C). `None` for the event of a
/// firing.
pub(super) fn of_event(event: &EventSummary) -> Option<Occurrence> {
    if OWN_EVENTS.iter().any(|own| event.message.starts_with(own)) {
        return None;
    }
    Some(Occurrence::Record {
        source: event.source.as_str().to_owned(),
        kind: event.severity.as_str().to_owned(),
        record: Reference::parse(&format!("event:{}", event.id)).ok()?,
        title: one_line(&event.message),
        class: Class::C,
        at: event.at,
    })
}

/// The expiries of the operational keys: the time that a key ends at its provider
/// (`keys.expiry`), and the time that the rotation of its active version is due
/// (`keys.rotation`).
pub(super) fn of_keys(keys: &[OperationalKey]) -> Vec<Occurrence> {
    let mut found = Vec::new();
    for key in keys {
        let Ok(record) = Reference::parse(&format!("operational_key:{}", key.id)) else {
            continue;
        };
        let mut push = |subject: &str, expires_at| {
            found.push(Occurrence::Expiry {
                subject: subject.to_owned(),
                record: record.clone(),
                title: one_line(&key.name),
                class: Class::C,
                expires_at,
            });
        };
        if let Some(expires_at) = key.metadata.expires_at {
            push(KEY_EXPIRY, expires_at);
        }
        let active = key
            .versions
            .iter()
            .find(|version| version.state == VersionState::Active);
        if let (Some(days), Some(active)) = (key.metadata.rotation_days, active) {
            push(
                KEY_ROTATION,
                active.created_at + TimeDelta::days(i64::from(days)),
            );
        }
    }
    found
}
