//! One named change to a READY repository (OD-E14; K-E1 Sections 13.2 and 13.6): the engine
//! sends it once, the held answers it made old are dropped, and every change that was sent is
//! recorded with what came of it before the caller hears the outcome.
//!
//! No transaction is open while the change is on the network: the record begins after GitHub's
//! answer. A change the engine did not send records nothing.

use async_graphql::{Context, Error as GraphQlError, Result};
use source_exchange::{Change, Receipt, Settled};

use super::{
    Target,
    failure::{self, Subject},
    scheduling, trail,
};
use crate::{
    api_errors::{commit, pool},
    engine::sources,
    fleet::events::Trail,
    work::projects::{Error, store::choices},
};

/// Sends `change` to `target` once and records it; the receipt when GitHub made it.
///
/// # Errors
/// The fixed words of [`failure::unsent`] when nothing was sent, of [`failure::settled`] for a
/// refusal, a limit or an unknown outcome, and `INTERNAL` when the record could not be written
/// twice ([`failure::unrecorded`] once GitHub made the change).
pub(in crate::work::projects) async fn make(
    ctx: &Context<'_>,
    target: &Target,
    change: &Change,
    subject: Subject,
) -> Result<Receipt> {
    failure::settled(send(ctx, target, change).await?, subject)
}

/// Sends `change` to `target` once and records it; what came of it, whatever GitHub answered.
///
/// # Errors
/// The fixed words of [`failure::unsent`] when nothing was sent, and `INTERNAL` when the record
/// could not be written twice ([`failure::unrecorded`] once GitHub made the change).
pub(in crate::work::projects) async fn send(
    ctx: &Context<'_>,
    target: &Target,
    change: &Change,
) -> Result<Settled> {
    let s = scheduling(ctx)?;
    let settled = sources::change(&s, target.account, change)
        .await
        .map_err(failure::unsent)?;
    for path in dropped(change) {
        sources::forget(&s.transport, target.account, &path);
    }
    if record(ctx, target, change, &settled).await.is_err() {
        // The cause reaches the process log with the second failure, if there is one.
        tracing::warn!(
            function = change.function(),
            "the record of a change to GitHub failed; trying once more"
        );
        let made = matches!(settled, Settled::Done(_));
        record(ctx, target, change, &settled)
            .await
            .map_err(|second| {
                if made {
                    failure::unrecorded(second)
                } else {
                    second
                }
            })?;
    }
    Ok(settled)
}

/// One audit entry and one event for a change that was sent, in one transaction.
async fn record(
    ctx: &Context<'_>,
    target: &Target,
    change: &Change,
    settled: &Settled,
) -> Result<(), GraphQlError> {
    let failed = |e: sqlx::Error| Error::from(e).into_graphql();
    let trail = Trail::from_context(ctx)?;
    let mut tx = pool(ctx)?.begin().await.map_err(failed)?;
    let slug = choices::repository_slug(&mut tx, target.repository)
        .await
        .map_err(failed)?;
    let entry = trail::entry(change, (target.repository, &slug), settled);
    trail
        .write(&mut tx, entry)
        .await
        .map_err(crate::api_errors::internal)?;
    commit(tx).await
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
