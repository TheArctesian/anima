//! One named change to a READY repository (OD-E14; K-E1 Sections 13.2 and 13.6): the engine
//! sends it once, the held answers it made old are dropped, and every change that was sent is
//! recorded with what came of it before the caller hears the outcome.
//!
//! The send, the forgetting and the record run to the end in a task of their own: a client that
//! goes away while GitHub works still leaves the record of the change GitHub made (as the VPN
//! broker's changes do, LOG-005). No transaction is open while the change is on the network:
//! the record begins after GitHub's answer. A change the engine did not send records nothing.

use async_graphql::{Context, Result};
use source_exchange::{Change, Receipt, Settled};
use tracing::Instrument;

use super::{
    Target,
    failure::{self, Subject},
    pace, scheduling, trail,
};
use crate::{
    api_errors::{internal, schema_dependency},
    engine::sources::{self, Scheduling},
    fleet::events::{ClientAddr, Operator, Trail},
    work::projects::store::choices,
};

/// Who made the change, for its record.
type Who = (Operator, ClientAddr);

/// Sends `change` to `target` once and records it; the receipt when GitHub made it.
///
/// # Errors
/// The fixed words of [`failure::unsent`] when nothing was sent, of [`failure::settled`] for a
/// refusal, a limit or an unknown outcome, and `INTERNAL` when the record could not be written
/// twice ([`failure::unrecorded`]).
pub(in crate::work::projects) async fn make(
    ctx: &Context<'_>,
    target: &Target,
    change: &Change,
    subject: Subject,
) -> Result<Receipt> {
    failure::settled(send(ctx, target, change).await?, subject)
}

/// Sends `change` to `target` once and records it; what came of it, whatever GitHub answered.
/// An API key's change keeps the sync's reserve and goes at the key's pace ([`pace`]).
///
/// # Errors
/// `UNAVAILABLE` past the key's pace, the fixed words of [`failure::unsent`] when nothing was
/// sent, and `INTERNAL` when the record could not be written twice ([`failure::unrecorded`]).
pub(in crate::work::projects) async fn send(
    ctx: &Context<'_>,
    target: &Target,
    change: &Change,
) -> Result<Settled> {
    let s = scheduling(ctx)?;
    let reserve = pace::reserve(ctx)?;
    let who = (
        schema_dependency::<Operator>(ctx)?.clone(),
        *schema_dependency::<ClientAddr>(ctx)?,
    );
    let (target, change) = (target.clone(), change.clone());
    let sent_and_recorded = tokio::spawn(
        async move {
            let settled = sources::change(&s, target.account, &change, reserve)
                .await
                .map_err(failure::unsent)?;
            for path in dropped(&change) {
                sources::forget(&s.transport, target.account, &path);
            }
            recorded(&s, &who, (&target, &change), &settled).await?;
            Ok(settled)
        }
        .in_current_span(),
    );
    sent_and_recorded.await.map_err(internal)?
}

/// Records what came of `change`, with one more try when the first fails.
///
/// # Errors
/// [`failure::unrecorded`] when both tries fail.
async fn recorded(
    s: &Scheduling,
    who: &Who,
    sent: (&Target, &Change),
    settled: &Settled,
) -> Result<()> {
    if record(s, who.clone(), sent, settled).await.is_ok() {
        return Ok(());
    }
    // The cause reaches the process log with the second failure, if there is one.
    tracing::warn!(
        function = sent.1.function(),
        "the record of a change to GitHub failed; trying once more"
    );
    record(s, who.clone(), sent, settled)
        .await
        .map_err(|second| failure::unrecorded(settled, second))
}

/// One audit entry and one event for a change that was sent, in one transaction, stamped when
/// it is written.
async fn record(
    s: &Scheduling,
    who: Who,
    (target, change): (&Target, &Change),
    settled: &Settled,
) -> Result<(), sqlx::Error> {
    let trail = Trail::new(&s.bus, who, s.clock.now());
    let mut tx = s.pool.begin().await?;
    let slug = choices::repository_slug(&mut tx, target.repository).await?;
    let entry = trail::entry(change, (target.repository, &slug), settled);
    trail
        .write(&mut tx, entry)
        .await
        .map_err(|crate::fleet::events::Error::Database(error)| error)?;
    tx.commit().await
}

/// The paths whose held answers `change` may have made old: each with what lies below it
/// (K-E1 Section 13.6).
fn dropped(change: &Change) -> Vec<String> {
    let repository = change.repository();
    let base = format!("/repos/{}/{}", repository.owner(), repository.name());
    let below: Vec<String> = match change {
        Change::CreateIssue { .. }
        | Change::AddIssueComment { .. }
        | Change::SetIssueOpen { .. } => vec!["/issues".to_owned()],
        Change::MergePullRequest { .. } => ["/pulls", "/commits", "/branches", "/actions/runs"]
            .map(str::to_owned)
            .to_vec(),
        Change::RerunWorkflowRun { .. }
        | Change::RerunFailedWorkflowJobs { .. }
        | Change::CancelWorkflowRun { .. }
        | Change::DispatchWorkflow { .. } => vec!["/actions/runs".to_owned()],
        Change::SetCommitStatus { sha, .. } => vec![format!("/commits/{sha}")],
    };
    below
        .into_iter()
        .map(|tail| format!("{base}{tail}"))
        .collect()
}
