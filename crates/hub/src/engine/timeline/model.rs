//! What the timeline shows: Evidence, its category and its basis (K-E3; TML-002 to TML-004).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;
use chrono::{DateTime, Utc};

use crate::engine::records::Reference;

/// The closed list of what an entry is about (TML-004). A new category needs a change to
/// the extensions plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "EvidenceCategory")]
pub enum Category {
    Sleep,
    Workout,
    Visit,
    Travel,
    Code,
    Message,
    Post,
    Media,
    Reading,
    Spending,
    Document,
    Device,
    Session,
    Note,
}

/// How the source knows what an entry says (TML-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
#[graphql(name = "EvidenceBasis")]
pub enum Basis {
    /// A device or a service recorded it when it happened.
    Observed,
    /// It is in a plan or a calendar; it did not necessarily happen.
    Planned,
    /// The operator said so.
    OperatorReported,
    /// The hub or a source derived it from other records.
    Inferred,
}

/// One entry of the timeline: a domain's record, seen as something that happened (TML-002).
/// An adapter makes it from a row when the timeline is read; nothing stores it (TML-001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// The record this entry is made from.
    pub reference: Reference,
    /// The key of the source that recorded it, such as `event_log` or a K-E1 source key.
    pub source: String,
    /// The rank of that source; the higher rank wins when two sources record one event.
    pub rank: u16,
    pub category: Category,
    /// One line for a list. It holds no secret.
    pub title: String,
    /// When the event happened, as the source gives it (TML-009).
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    /// When the hub received the record, if that is later than the source wrote it, such as a
    /// delayed import. It never replaces `start` (TML-009).
    pub received_at: Option<DateTime<Utc>>,
    pub basis: Basis,
    /// What the adapter does not know about this entry, one sentence each (TML-006).
    pub uncertainty: Vec<String>,
    /// Equal for two entries of one category exactly when they record the same event;
    /// `None` when the adapter knows no other record of it.
    pub occurrence: Option<String>,
    /// The higher-ranked entry of the same event (TML-005). The timeline fills it.
    pub superseded_by: Option<Reference>,
}

/// A place in the newest-first order: by start, then by reference text, both descending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    pub start: DateTime<Utc>,
    pub reference: Reference,
}

impl Evidence {
    #[must_use]
    pub fn mark(&self) -> Mark {
        Mark {
            start: self.start,
            reference: self.reference.clone(),
        }
    }
}

impl Mark {
    /// True when `self` comes later in the newest-first order than `other`.
    #[must_use]
    pub fn is_after(&self, other: &Self) -> bool {
        (self.start, self.reference.as_str()) < (other.start, other.reference.as_str())
    }
}

/// What a reader asks an adapter for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    /// Only entries that start at or after this instant.
    pub from: DateTime<Utc>,
    /// Only entries that start before this instant.
    pub to: DateTime<Utc>,
    /// Only entries after this place in the newest-first order.
    pub after: Option<Mark>,
    /// Only these categories; `None` for all, an empty list for none.
    pub categories: Option<Vec<Category>>,
    /// The most entries to return.
    pub limit: usize,
}

impl Ask {
    /// True when `entry` is one this ask wants.
    #[must_use]
    pub fn wants(&self, entry: &Evidence) -> bool {
        self.from <= entry.start
            && entry.start < self.to
            && (self.after.as_ref()).is_none_or(|mark| entry.mark().is_after(mark))
            && (self.categories.as_ref()).is_none_or(|list| list.contains(&entry.category))
    }
}
