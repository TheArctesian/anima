//! The hash chain of the audit log (LOG-021 to LOG-028): each entry gets one link that holds
//! the hash of the link before it, so a change to an old entry, or its removal, shows. The
//! hashing and the check are pure (`link`, `check`); `store::chain` does the SQL.

mod canonical;
mod check;
mod link;
#[cfg(test)]
mod tests;

use chrono::{DateTime, Days, Utc};
use sqlx::{PgConnection, PgPool};

pub(super) use check::{ChainCheck, Fault, FaultKind, Walk};
pub(super) use link::{Hash, Link, StoredEntry, hex};

use super::{
    Bus, Error, NewEvent, Severity, Source, actor::ActorKind, record, retention::Log,
    store::chain as store,
};

/// Links `entry`, which the caller's transaction has just stored, after the newest link
/// (LOG-021, LOG-024). The lock of the chain stays until the caller commits or rolls back.
pub(super) async fn append(
    conn: &mut PgConnection,
    entry: &StoredEntry,
) -> Result<(), sqlx::Error> {
    store::lock(conn).await?;
    let (seq, previous) = next(store::head(conn).await?);
    store::insert_links(conn, &[Link::new(seq, previous, entry)]).await
}

/// The sequence number and previous hash of the link after `head`.
fn next(head: Option<(i64, Hash)>) -> (i64, Hash) {
    head.map_or((1, link::FIRST_PREVIOUS), |(seq, hash)| (seq + 1, hash))
}

/// Links each stored entry, oldest first, when the chain is empty, and returns how many it
/// linked (LOG-026). Call it once at start, before the hub serves or starts a task: after
/// that, the hub links only the entries it writes. With a chain that has links, it links
/// nothing, so an entry that the hub did not write stays without a link and the check finds
/// it.
///
/// # Errors
///
/// `Error::Database` if an entry cannot be read or a link cannot be written; nothing is linked.
pub async fn link_existing(db: &PgPool) -> Result<u64, Error> {
    let mut tx = db.begin().await?;
    store::lock(&mut tx).await?;
    let mut head = store::head(&mut tx).await?;
    if head.is_some() {
        return Ok(0);
    }
    let mut linked = 0;
    let mut after = None;
    loop {
        let entries = store::entries_after(&mut tx, after).await?;
        let Some(last) = entries.last() else { break };
        after = Some((last.at, last.id));
        let links: Vec<Link> = entries
            .iter()
            .map(|entry| {
                let (seq, previous) = next(head);
                let link = Link::new(seq, previous, entry);
                head = Some((link.seq, link.hash));
                link
            })
            .collect();
        store::insert_links(&mut tx, &links).await?;
        linked += links.len() as u64;
    }
    tx.commit().await?;
    Ok(linked)
}

/// Checks the chain at `now` (LOG-027). It changes nothing. A link without its entry is a
/// fault only when the entry's UTC day is inside the retention of the audit log at `now`.
///
/// # Errors
///
/// `Error::Database` if the links or the entries cannot be read.
pub(super) async fn check_chain(db: &PgPool, now: DateTime<Utc>) -> Result<ChainCheck, Error> {
    // The audit log keeps one year whatever the event period is (DB-005).
    let mut walk = Walk::new(Log::Audit.first_kept_day(now, Days::new(0)));
    let unlinked = store::walk(db, &mut walk).await?;
    Ok(walk.finish(unlinked))
}

/// The daily check (LOG-028): when the chain has a fault, writes an Error event of the source
/// Hub. It never repairs the chain.
///
/// # Errors
///
/// `Error::Database` if the check cannot run or the event cannot be written.
pub async fn check_chain_daily(db: &PgPool, bus: &Bus, now: DateTime<Utc>) -> Result<(), Error> {
    let Some(fault) = check_chain(db, now).await?.fault else {
        return Ok(());
    };
    let event = NewEvent {
        severity: Severity::Error,
        source: Source::Hub,
        device: None,
        message: fault_message(&fault),
        actor: ActorKind::Hub,
    };
    let mut conn = db.acquire().await?;
    record(&mut conn, bus, now, event).await.map(drop)
}

/// The sentence of the daily event for `fault`.
fn fault_message(fault: &Fault) -> String {
    let what = match fault.kind {
        FaultKind::Changed => "an entry was changed",
        FaultKind::Missing => "an entry was removed",
        FaultKind::Broken => "a link is missing or wrong",
        FaultKind::NotLinked => "an entry has no link",
    };
    let place = fault.seq.map_or_else(
        || format!("entry {}", fault.entry_id),
        |seq| format!("link {seq}"),
    );
    format!("The audit log chain is broken at {place}: {what}.")
}
