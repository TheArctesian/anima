//! The check of the chain (LOG-027): a walk over the links in the order of the sequence
//! number, which stops at the first fault. Pure: the store feeds it one link at a time.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::link::{FIRST_PREVIOUS, Hash, Link, StoredEntry, hash};

/// What the check found (LOG-027).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::events) struct ChainCheck {
    /// The links that the check read.
    pub links: u64,
    /// The links whose entry the retention dropped.
    pub dropped: u64,
    /// The sequence number and hash of the newest link that the check read.
    pub head: Option<(i64, Hash)>,
    /// The first fault, or `None` when the chain is whole.
    pub fault: Option<Fault>,
}

/// The first place where the chain and the entries do not agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::events) struct Fault {
    pub kind: FaultKind,
    /// `None` for an entry that has no link.
    pub seq: Option<i64>,
    pub entry_id: Uuid,
    pub at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::fleet::events) enum FaultKind {
    /// The entry does not give the hash of its link: a field changed, or entries changed places.
    Changed,
    /// The link has no entry, and the day of the entry is inside the retention.
    Missing,
    /// A sequence number is missing, or a previous hash is not the hash of the link before.
    Broken,
    /// An entry has no link: the hub did not write it.
    NotLinked,
}

/// The state of the walk between two links.
pub(in crate::fleet::events) struct Walk {
    first_kept_day: NaiveDate,
    links: u64,
    dropped: u64,
    head: Option<(i64, Hash)>,
    fault: Option<Fault>,
}

impl Walk {
    /// A walk that accepts a link without its entry only for an entry of a UTC day before
    /// `first_kept_day`, which the retention may have dropped.
    pub(in crate::fleet::events) fn new(first_kept_day: NaiveDate) -> Self {
        Self {
            first_kept_day,
            links: 0,
            dropped: 0,
            head: None,
            fault: None,
        }
    }

    /// Checks the next link and its entry, if the entry exists. Returns `false` at the first
    /// fault: the walk then reads no more links.
    pub(in crate::fleet::events) fn step(
        &mut self,
        link: &Link,
        entry: Option<&StoredEntry>,
    ) -> bool {
        let (seq, previous) = self
            .head
            .map_or((1, FIRST_PREVIOUS), |(seq, hash)| (seq + 1, hash));
        let kind = if link.seq != seq || link.previous != previous {
            Some(FaultKind::Broken)
        } else {
            self.entry_fault(link, entry)
        };
        if let Some(kind) = kind {
            self.fault = Some(Fault {
                kind,
                seq: Some(link.seq),
                entry_id: link.entry_id,
                at: link.entry_at,
            });
            return false;
        }
        self.links += 1;
        self.head = Some((link.seq, link.hash));
        true
    }

    fn entry_fault(&mut self, link: &Link, entry: Option<&StoredEntry>) -> Option<FaultKind> {
        match entry {
            Some(entry) if hash(link.seq, &link.previous, entry) == link.hash => None,
            Some(_) => Some(FaultKind::Changed),
            None if link.entry_at.date_naive() < self.first_kept_day => {
                self.dropped += 1;
                None
            }
            None => Some(FaultKind::Missing),
        }
    }

    /// The result, with the oldest entry that has no link when the links had no fault.
    pub(in crate::fleet::events) fn finish(
        self,
        unlinked: Option<(Uuid, DateTime<Utc>)>,
    ) -> ChainCheck {
        let fault = self.fault.or_else(|| {
            unlinked.map(|(entry_id, at)| Fault {
                kind: FaultKind::NotLinked,
                seq: None,
                entry_id,
                at,
            })
        });
        ChainCheck {
            links: self.links,
            dropped: self.dropped,
            head: self.head,
            fault,
        }
    }
}
