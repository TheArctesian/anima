//! The report of a holder (K10 Section 7), and the end of what a machine lost: a lease that
//! ran out and an attempt that is too long.

use chrono::{DateTime, Utc};
use daemon_core::wire::{Lease, Reported, StopOrder, WorkReport, WorkState};
use sqlx::PgConnection;

use super::{
    Error, Queue, Reporting, ended,
    lifecycle::{self, Refusal},
    model::Fault,
    placement, store, trail,
};
use crate::{
    fleet::events::{Bus, ClientAddr},
    ids::{DeviceId, JobId},
};

/// Answers the report of `device` at `now` (K10 Section 7): the new state, the log part, the
/// new lease and a stop when there is one.
///
/// # Errors
/// `Database` or `Events` when a change cannot be written; nothing is changed then.
pub async fn report(
    queue: Queue<'_>,
    device: DeviceId,
    report: &WorkReport,
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Reporting, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = report_in(&mut tx, queue.bus, (device, report), (client, now)).await;
    ended(tx, outcome).await
}

async fn report_in(
    conn: &mut PgConnection,
    bus: &Bus,
    (device, report): (DeviceId, &WorkReport),
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Reporting, Error> {
    if let Some(machine) = store::machine_locked(conn, device).await?
        && placement::is_off(&machine, now)
    {
        return Ok(Reporting::Off);
    }
    let Some(job) = store::job_locked(conn, JobId::from(report.id)).await? else {
        return Ok(Reporting::NotHolder);
    };
    let mut next = match lifecycle::reported(&job, device, report, now) {
        Ok(next) => next,
        Err(Refusal::NotHolder) => return Ok(Reporting::NotHolder),
        Err(Refusal::Malformed) => return Ok(Reporting::Malformed),
    };
    if let Some(part) = &report.log
        && lifecycle::log_fits(&job, part.from, part.text.len())
    {
        store::append_log(conn, job.id, part.from, &trail::clean_log(&part.text)).await?;
        next.log_end = job.log_end + part.text.len() as u64;
    }
    store::save(conn, &next).await?;
    if report.state == WorkState::Refused {
        fault(conn, bus, (device, Fault::Refused), now).await?;
        trail::attempt_ended(conn, &next, (device, "refused"), (client, now)).await?;
    } else if next.state.is_final() {
        trail::attempt_ended(conn, &next, (device, next.state.as_str()), (client, now)).await?;
    }
    Ok(Reporting::Reported(Reported {
        lease: Lease {
            until: next.lease_ends_at.unwrap_or(now),
        },
        stop: next.stop.map(|reason| StopOrder { reason }),
        log_end: Some(next.log_end),
    }))
}

/// Counts a job that the machine refused or lost; the fifth in 1 hour gives the machine no
/// work for 1 hour, with an event (K10 Section 11, SF-011).
async fn fault(
    conn: &mut PgConnection,
    bus: &Bus,
    (device, fault): (DeviceId, Fault),
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let recent =
        store::add_fault(conn, (device, fault), now, now - placement::FAULT_WINDOW).await?;
    if let Some(until) = placement::off_until(recent, now) {
        store::set_off_until(conn, device, until).await?;
        trail::set_off(conn, bus, device, now).await?;
    }
    Ok(())
}

/// Ends each lease that ran out by `now` and each attempt that holds a job for longer than its
/// limit (K10 Section 7); the upkeep calls it every 30 seconds. Returns how many jobs changed.
///
/// # Errors
/// `Database` or `Events` when a change cannot be written; nothing is changed then.
pub async fn end_lost(queue: Queue<'_>, now: DateTime<Utc>) -> Result<usize, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = end_lost_in(&mut tx, queue.bus, now).await;
    ended(tx, outcome).await
}

async fn end_lost_in(
    conn: &mut PgConnection,
    bus: &Bus,
    now: DateTime<Utc>,
) -> Result<usize, Error> {
    let client = trail::HUB_ADDRESS;
    let mut changed = 0;
    for job in store::ended_leases(conn, now).await? {
        let Some(device) = job.holder else { continue };
        let next = lifecycle::lease_lost(&job, now);
        store::save(conn, &next).await?;
        fault(conn, bus, (device, Fault::Lost), now).await?;
        trail::attempt_ended(conn, &next, (device, "lease_ended"), (client, now)).await?;
        changed += 1;
    }
    for job in store::over_time(conn, now).await? {
        let Some(device) = job.holder else { continue };
        if !lifecycle::overdue(&job, now) {
            continue;
        }
        let next = lifecycle::expired(&job, now);
        store::save(conn, &next).await?;
        trail::attempt_ended(conn, &next, (device, "expired"), (client, now)).await?;
        changed += 1;
    }
    Ok(changed)
}
