//! One sync run of one account (SRC-005, SRC-007, SRC-012, EXT-002; K-E1 Section 8).
//!
//! The order is fixed: claim, then for each intake whose extension is on, fetch a page and
//! store it with the new slice in one transaction that first checks the claim; then end. A run
//! whose claim was superseded writes nothing more.

mod ending;
mod fetch;
mod page;

use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use source_exchange::Declaration;
use uuid::Uuid;

use super::{
    Error, Scheduling,
    acquisition::transport::{Answer, Content},
    intake::Intake,
    model::{Account, ErrorKind, Outcome, Trigger},
    store::{self, runs},
};
use crate::{engine::extensions, ids::SyncRunId};

/// How long a claim holds without a stored page.
pub const CLAIM_FOR: TimeDelta = TimeDelta::minutes(10);
/// The most requests one run sends; the next run resumes.
pub const MAX_REQUESTS: u32 = 2000;

/// A failed run: its kind, fixed words for the operator, and the end of a rate limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub kind: ErrorKind,
    pub words: &'static str,
    pub until: Option<DateTime<Utc>>,
}

impl Failure {
    const fn new(kind: ErrorKind, words: &'static str) -> Self {
        Self {
            kind,
            words,
            until: None,
        }
    }

    const fn store() -> Self {
        Self::new(ErrorKind::Store, "The hub could not store what it read.")
    }
}

/// How the work of a run stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Done,
    /// The consumer was set off, the account paused, or the request cap reached.
    Stopped,
    Failed(Failure),
    /// Another run claimed the account, or it was removed: write nothing more.
    LostClaim,
}

/// A claimed account: what the run works on.
struct Claim {
    account: Account,
    state: Value,
    generation: i64,
    run: SyncRunId,
    declaration: Option<Declaration>,
}

/// Runs a sync of `account` started by `trigger`. `None` when another run holds the account,
/// it is paused, or the run lost its claim; otherwise how it ended.
///
/// # Errors
/// `Database` when the claim or the end cannot be written.
pub async fn run(
    scheduling: &Scheduling,
    account: crate::ids::SourceAccountId,
    trigger: Trigger,
) -> Result<Option<Outcome>, Error> {
    let Some(claim) = claim(scheduling, account, trigger).await? else {
        return Ok(None);
    };
    scheduling.feed.publish(account);
    let stop = match claim.declaration {
        Some(declaration) => sync(scheduling, &claim, &declaration).await,
        None => Stop::Failed(Failure::new(
            ErrorKind::Provider,
            "The hub no longer knows this source.",
        )),
    };
    ending::finish(scheduling, &claim, stop).await
}

/// Claims the account, ends the runs a superseded claim left open, and starts the run row, in
/// one transaction.
async fn claim(
    scheduling: &Scheduling,
    id: crate::ids::SourceAccountId,
    trigger: Trigger,
) -> Result<Option<Claim>, Error> {
    let now = scheduling.clock.now();
    let mut tx = scheduling.pool.begin().await?;
    let Some(claimed) = store::claim(&mut tx, id, (now, now + CLAIM_FOR)).await? else {
        return Ok(None);
    };
    let run = SyncRunId::from(Uuid::now_v7());
    runs::supersede(&mut tx, id, now).await?;
    runs::start(&mut tx, (run, id), trigger, now).await?;
    tx.commit().await?;
    let declaration = scheduling
        .registry
        .declaration(&claimed.account.source)
        .copied();
    Ok(Some(Claim {
        generation: claimed.account.run_generation,
        account: claimed.account,
        state: claimed.sync_state.0,
        run,
        declaration,
    }))
}

/// Feeds every intake of the source whose extension is on, in wiring order.
async fn sync(scheduling: &Scheduling, claim: &Claim, declaration: &Declaration) -> Stop {
    let Ok(on) = extensions::on_keys(&scheduling.pool).await else {
        return Stop::Failed(Failure::store());
    };
    let intakes: Vec<Arc<dyn Intake>> = scheduling
        .registry
        .intakes_of(&claim.account.source)
        .filter(|intake| on.contains(&intake.extension()))
        .cloned()
        .collect();
    let mut sent = 0;
    for intake in intakes {
        match feed(scheduling, (claim, declaration), intake.as_ref(), &mut sent).await {
            Stop::Done => {}
            other => return other,
        }
    }
    Stop::Done
}

/// Why a page that did not move the sync on fails the run. A reply too large to read is named
/// as such: the provider found it too large for its request (the listing, say).
fn stuck(answer: &Answer) -> Failure {
    let words = match answer.content {
        Content::TooLarge => "The source sent a reply larger than the hub reads.",
        Content::Body(_) => "The source gave a page that did not move the sync on.",
    };
    Failure::new(ErrorKind::Provider, words)
}

/// Feeds one intake until it has no request left, from its stored slice.
async fn feed(
    scheduling: &Scheduling,
    (claim, declaration): (&Claim, &Declaration),
    intake: &dyn Intake,
    sent: &mut u32,
) -> Stop {
    let stored = claim.state.get(intake.slice_key()).cloned();
    let mut slice = intake.begin(&stored.unwrap_or_else(|| json!({})));
    loop {
        if *sent >= MAX_REQUESTS {
            return Stop::Stopped;
        }
        match extensions::on_keys(&scheduling.pool).await {
            Ok(on) if on.contains(&intake.extension()) => {}
            Ok(_) => return Stop::Stopped,
            Err(_) => return Stop::Failed(Failure::store()),
        }
        let Some(request) = intake.next_request(&slice) else {
            return Stop::Done;
        };
        *sent += 1;
        let answer = match fetch::fetch(scheduling, (claim, declaration), &request).await {
            Ok(answer) => answer,
            Err(failure) => return Stop::Failed(failure),
        };
        let fetched = page::Fetched {
            request: &request,
            answer: &answer,
            slice: &slice,
        };
        match page::store(scheduling, claim, intake, fetched).await {
            page::Stored::Moved(next) if next == slice => return Stop::Failed(stuck(&answer)),
            page::Stored::Moved(next) => slice = next,
            page::Stored::Stop(stop) => return stop,
        }
    }
}
