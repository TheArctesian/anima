//! The claim of a machine (K10 Section 4): what the machine said is recorded, the attempts
//! whose answer was lost end, and the placement chooses the job.

use chrono::{DateTime, Utc};
use daemon_core::wire::{Claim, Lease, Switch, WorkJob};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{
    Claimed, Error, Queue, ended, lifecycle,
    model::{Job, Machine},
    placement::{self, Around, Offer},
    store, trail,
};
use crate::{
    fleet::events::{Bus, ClientAddr},
    ids::{DeviceId, JobId},
};

/// Answers the claim of `device` at `now` (K10 Section 4): records what the machine said, ends
/// the attempts whose answer was lost, and gives the machine the job that the placement
/// chooses. `claim.device` is already known to be `device`.
///
/// # Errors
/// `Database` or `Events` when a change cannot be written; nothing is changed then.
pub async fn claim(
    queue: Queue<'_>,
    (device, name): (DeviceId, &str),
    claim: &Claim,
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Claimed, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = claim_in(&mut tx, queue.bus, (device, name, claim), (client, now)).await;
    ended(tx, outcome).await
}

async fn claim_in(
    conn: &mut PgConnection,
    bus: &Bus,
    (device, name, claim): (DeviceId, &str, &Claim),
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Claimed, Error> {
    let known = store::machine_locked(conn, device).await?;
    let first_on = claim.switch == Switch::On && known.is_none_or(|old| old.first_on_at.is_none());
    let machine = Machine {
        device,
        switch: claim.switch,
        unclean: claim.unclean,
        off_by_operator: known.is_some_and(|old| old.off_by_operator),
        off_until: known.and_then(|old| old.off_until),
        first_on_at: known
            .and_then(|old| old.first_on_at)
            .or(first_on.then_some(now)),
        excluded: known.is_some_and(|old| old.excluded),
    };
    store::save_machine(conn, &machine, now).await?;
    store::save_said(conn, (device, name), claim).await?;
    if first_on {
        trail::first_claim(conn, bus, (device, name), now).await?;
    }
    if placement::is_off(&machine, now) {
        return Ok(Claimed::Off);
    }
    let mut holds_one = false;
    for held in store::held_by(conn, device).await? {
        let named = claim
            .holds
            .iter()
            .any(|each| JobId::from(each.id) == held.id && each.attempt == held.attempt);
        if named {
            holds_one = true;
        } else {
            // The answer to the claim of this attempt was lost (SF-011).
            let back = lifecycle::answer_lost(&held, now);
            store::save(conn, &back).await?;
            trail::attempt_ended(conn, &back, (device, "answer_lost"), (client, now)).await?;
        }
    }
    if holds_one || !placement::can_take(claim) {
        return Ok(Claimed::Nothing);
    }
    let offer = Offer::of(claim, &machine);
    place(conn, (&offer, name), (client, now)).await
}

/// Decides each queued job for the machine of `offer`, records each decision, and gives the
/// machine the job that the placement chooses, with the audit entry of the decision (ROU-005).
async fn place(
    conn: &mut PgConnection,
    (offer, name): (&Offer, &str),
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Claimed, Error> {
    let queued = store::oldest_queued(conn).await?;
    let pins = store::pins(conn).await?;
    let asking = (now - placement::FRESH, now);
    let others = store::others_asking(conn, offer.device, asking).await?;
    let around = Around {
        pins: &pins,
        others: &others,
        now,
    };
    let verdicts: Vec<_> = queued
        .iter()
        .map(|job| placement::verdict(job, offer, &around))
        .collect();
    let decided: Vec<_> = queued
        .iter()
        .map(|job| job.id)
        .zip(verdicts.iter().copied())
        .collect();
    store::save_decisions(conn, (offer, name), &decided, now).await?;
    let Some(chosen) = placement::choose(&queued, &verdicts, offer.device) else {
        return Ok(Claimed::Nothing);
    };
    let job = lifecycle::claimed(chosen, offer.device, now);
    store::save(conn, &job).await?;
    let decisions = store::decisions_of(conn, job.id).await?;
    trail::attempt_started(conn, &job, (offer.device, &decisions), (client, now)).await?;
    Ok(Claimed::Job(Box::new(wire_job(&job))))
}

/// The job as the machine gets it.
fn wire_job(job: &Job) -> WorkJob {
    WorkJob {
        id: job.id.into(),
        work_type: job.asked.work_type,
        kind: job.asked.kind,
        attempt: job.attempt,
        lease: Lease {
            until: job.lease_ends_at.unwrap_or(job.changed_at),
        },
        limits: job.asked.limits,
        project: job.asked.project.map(Uuid::from),
        repository: job.asked.repository.clone(),
        secrets: job.asked.secrets.clone(),
        parameters: job.asked.parameters.clone(),
    }
}
