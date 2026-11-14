//! One page in one transaction: check the claim, let the intake store the records, write the
//! new slice and the counts, extend the claim, commit (SRC-007). The slice moves only when the
//! records of its page are stored.

use serde_json::Value;
use source_exchange::{Body, Request, Response};

use super::{CLAIM_FOR, Claim, Failure, Stop};
use crate::engine::sources::{
    Scheduling,
    acquisition::transport::{Answer, Content},
    intake::{Intake, Page},
    store::{
        self,
        runs::{self, StoredProblem},
    },
};

/// A fetched page and the slice it was asked from.
pub(super) struct Fetched<'a> {
    pub request: &'a Request,
    pub answer: &'a Answer,
    pub slice: &'a Value,
}

/// What came of storing a page.
pub(super) enum Stored {
    /// Committed; the intake's slice is now this.
    Moved(Value),
    /// Nothing was stored, and the run stops so.
    Stop(Stop),
}

/// Stores one page for `intake`. Paused since the claim: nothing is written and the run stops.
/// A claim another run took: nothing is written and the run loses it. A failure of the intake
/// or of a write rolls the whole page back, so the stored slice still asks for this page.
pub(super) async fn store(
    scheduling: &Scheduling,
    claim: &Claim,
    intake: &dyn Intake,
    fetched: Fetched<'_>,
) -> Stored {
    match write(scheduling, claim, intake, fetched).await {
        Ok(stored) => stored,
        Err(_) => Stored::Stop(Stop::Failed(Failure::store())),
    }
}

async fn write(
    scheduling: &Scheduling,
    claim: &Claim,
    intake: &dyn Intake,
    fetched: Fetched<'_>,
) -> Result<Stored, crate::engine::sources::Error> {
    let now = scheduling.clock.now();
    let id = claim.account.id;
    let mut tx = scheduling.pool.begin().await?;
    match store::lock_claim(&mut tx, id, claim.generation).await? {
        None => return Ok(Stored::Stop(Stop::LostClaim)),
        Some(true) => return Ok(Stored::Stop(Stop::Stopped)),
        Some(false) => {}
    }
    let page = Page {
        account: id,
        request: fetched.request,
        response: Response {
            status: fetched.answer.status,
            body: match &fetched.answer.content {
                Content::Body(body) => Body::Read(body),
                Content::TooLarge => Body::TooLarge,
            },
        },
        slice: fetched.slice,
        now,
    };
    let taken = intake.take(&mut tx, page).await?;
    let problems: Vec<StoredProblem> = taken
        .problems
        .iter()
        .map(|problem| StoredProblem {
            code: problem.code.to_owned(),
            subject: problem.subject.clone(),
            message: problem.message.clone(),
        })
        .collect();
    store::write_slice(
        &mut tx,
        id,
        (intake.slice_key(), &taken.slice),
        now + CLAIM_FOR,
    )
    .await?;
    runs::add(&mut tx, claim.run, taken.counts, &problems).await?;
    tx.commit().await?;
    Ok(Stored::Moved(taken.slice))
}
