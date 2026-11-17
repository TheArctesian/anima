//! The timeline: the records of every domain as Evidence, newest first (TML-001 to TML-007).
//! It has no table. Each domain gives an adapter and the timeline asks them when it is read.

mod adapter;
mod cursor;
mod event_log;
pub mod graphql;
mod model;
mod ranking;
#[cfg(test)]
mod tests;

use sqlx::PgPool;

pub use adapter::{Adapter, Adapters, Unavailable};
pub use event_log::event_log;
pub use model::{Ask, Basis, Category, Evidence, Mark};

/// Why the timeline could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Unavailable(#[from] Unavailable),
}

/// One page of the timeline and whether more entries follow it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub entries: Vec<Evidence>,
    pub has_next_page: bool,
}

/// The newest-first order of the timeline.
fn newest_first(a: &Evidence, b: &Evidence) -> std::cmp::Ordering {
    (b.start, b.reference.as_str()).cmp(&(a.start, a.reference.as_str()))
}

/// One page from what the adapters returned: the entries `ask` wants, newest first, each
/// record once, with `superseded_by` set (TML-005), cut to `ask.limit`. An entry that
/// supersedes another is found only among the entries read for this page.
fn arrange(mut entries: Vec<Evidence>, ask: &Ask) -> Page {
    entries.retain(|entry| ask.wants(entry));
    entries.sort_by(newest_first);
    entries.dedup_by(|a, b| a.reference == b.reference);
    ranking::supersede(&mut entries);
    let has_next_page = entries.len() > ask.limit;
    entries.truncate(ask.limit);
    Page {
        entries,
        has_next_page,
    }
}

/// Reads one page: asks every adapter for one entry more than `ask.limit`, so the page knows
/// whether another follows.
///
/// # Errors
/// `Unavailable` when an adapter cannot read its records; no partial page is returned.
pub async fn read(adapters: &Adapters, db: &PgPool, ask: &Ask) -> Result<Page, Error> {
    let wider = Ask {
        limit: ask.limit.saturating_add(1),
        ..ask.clone()
    };
    let mut entries = Vec::new();
    for adapter in adapters.iter() {
        entries.extend(adapter.evidence(db, &wider).await?);
    }
    Ok(arrange(entries, ask))
}
