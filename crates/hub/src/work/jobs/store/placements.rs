//! What the placement reads and writes in schema `jobs`: what each machine said in its last
//! claim, the pins and the exclusions of the operator, and the last decision for each job.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use daemon_core::wire::Claim;
use sqlx::PgConnection;

use super::{count, read_word, stored, word};
use crate::{
    ids::{DeviceId, JobId, ProjectId},
    work::jobs::{
        Error,
        placement::{Offer, Verdict},
        reasons::Decision,
    },
};

fn words<T: serde::Serialize>(all: &[T]) -> Result<Vec<String>, Error> {
    all.iter().map(word).collect()
}

/// The words that this hub knows; a word of a newer daemon is dropped, so it fits no job.
fn known<T: serde::de::DeserializeOwned>(all: &[String]) -> Vec<T> {
    all.iter().filter_map(|text| read_word(text).ok()).collect()
}

/// Writes what the machine `name` said in its claim at `now`, for the claims of the others.
pub(in crate::work::jobs) async fn save_said(
    conn: &mut PgConnection,
    (device, name): (DeviceId, &str),
    claim: &Claim,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE jobs.machines SET name = $2, types = $3, kinds = $4, os = $5, architecture = $6, \
         free_slots = $7, free_memory_bytes = $8, free_disk_bytes = $9 WHERE device_id = $1",
    )
    .bind(device)
    .bind(name)
    .bind(words(&claim.types)?)
    .bind(words(&claim.kinds)?)
    .bind(word(&claim.os)?)
    .bind(word(&claim.architecture)?)
    .bind(i64::from(claim.free.slots))
    .bind(stored(claim.free.memory_bytes)?)
    .bind(stored(claim.free.disk_bytes)?)
    .execute(conn)
    .await?;
    Ok(())
}

type OfferRow = (
    DeviceId,
    Vec<String>,
    Vec<String>,
    String,
    String,
    i64,
    i64,
    bool,
);

fn offer(
    (device, types, kinds, os, architecture, memory, disk, excluded): OfferRow,
) -> Result<Offer, Error> {
    Ok(Offer {
        device,
        types: known(&types),
        kinds: known(&kinds),
        os: read_word(&os).ok(),
        architecture: read_word(&architecture).ok(),
        memory_bytes: count(memory)?,
        disk_bytes: count(disk)?,
        excluded,
    })
}

const SELECT_OFFER: &str = "SELECT device_id, types, kinds, os, architecture, free_memory_bytes, \
     free_disk_bytes, excluded FROM jobs.machines ";

/// What the last claim of the machine said; `None` before its first claim.
pub(in crate::work::jobs) async fn offer_of(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<Option<Offer>, Error> {
    let sql = format!("{SELECT_OFFER}WHERE device_id = $1");
    let row: Option<OfferRow> = sqlx::query_as(&sql)
        .bind(device)
        .fetch_optional(conn)
        .await?;
    row.map(offer).transpose()
}

/// The machines but `device` whose last claim is not older than `since` and that can take a
/// job at `now`: the two switches are on, the machine is clean and it has a free slot.
pub(in crate::work::jobs) async fn others_asking(
    conn: &mut PgConnection,
    device: DeviceId,
    (since, now): (DateTime<Utc>, DateTime<Utc>),
) -> Result<Vec<Offer>, Error> {
    let sql = format!(
        "{SELECT_OFFER}WHERE device_id <> $1 AND last_claim_at >= $2 AND switch = 'on' \
         AND NOT unclean AND free_slots >= 1 AND NOT off_by_operator \
         AND (off_until IS NULL OR off_until <= $3) ORDER BY device_id"
    );
    let rows: Vec<OfferRow> = sqlx::query_as(&sql)
        .bind(device)
        .bind(since)
        .bind(now)
        .fetch_all(conn)
        .await?;
    rows.into_iter().map(offer).collect()
}

/// The machine that the operator pinned for each project.
pub(in crate::work::jobs) async fn pins(
    conn: &mut PgConnection,
) -> Result<HashMap<ProjectId, DeviceId>, Error> {
    let rows: Vec<(ProjectId, DeviceId)> =
        sqlx::query_as("SELECT project_id, device_id FROM jobs.project_machines")
            .fetch_all(conn)
            .await?;
    Ok(rows.into_iter().collect())
}

/// Pins `device` for the project. Returns false when there is no such device: nothing is
/// written then.
pub(in crate::work::jobs) async fn pin(
    conn: &mut PgConnection,
    project: ProjectId,
    device: DeviceId,
) -> Result<bool, Error> {
    let done = sqlx::query(
        "INSERT INTO jobs.project_machines (project_id, device_id) \
         SELECT $1, id FROM devices.devices WHERE id = $2 \
         ON CONFLICT (project_id) DO UPDATE SET device_id = $2",
    )
    .bind(project)
    .bind(device)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Removes the pin of the project and returns the machine that was pinned.
pub(in crate::work::jobs) async fn unpin(
    conn: &mut PgConnection,
    project: ProjectId,
) -> Result<Option<DeviceId>, Error> {
    let old: Option<(DeviceId,)> = sqlx::query_as(
        "DELETE FROM jobs.project_machines WHERE project_id = $1 RETURNING device_id",
    )
    .bind(project)
    .fetch_optional(conn)
    .await?;
    Ok(old.map(|(device,)| device))
}

/// Sets the exclusion of the machine. Returns false for a machine that never asked for work.
pub(in crate::work::jobs) async fn set_excluded(
    conn: &mut PgConnection,
    device: DeviceId,
    excluded: bool,
) -> Result<bool, Error> {
    let done = sqlx::query("UPDATE jobs.machines SET excluded = $2 WHERE device_id = $1")
        .bind(device)
        .bind(excluded)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Writes the decision of the machine of `offer` for each job at `now`. A decision that did not
/// change is written again only after 1 minute, so that a machine that asks often writes little.
pub(in crate::work::jobs) async fn save_decisions(
    conn: &mut PgConnection,
    (offer, machine): (&Offer, &str),
    decided: &[(JobId, Verdict)],
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let jobs: Vec<JobId> = decided.iter().map(|(job, _)| *job).collect();
    let verdicts: Vec<&str> = decided
        .iter()
        .map(|(_, verdict)| verdict.as_str())
        .collect();
    sqlx::query(
        "INSERT INTO jobs.decisions AS old \
         (job_id, device_id, machine, at, verdict, free_memory_bytes, free_disk_bytes) \
         SELECT job, $2, $3, $4, verdict, $5, $6 FROM unnest($1::uuid[], $7::text[]) AS new (job, verdict) \
         ON CONFLICT (job_id, device_id) DO UPDATE SET machine = $3, at = $4, \
         verdict = excluded.verdict, free_memory_bytes = $5, free_disk_bytes = $6 \
         WHERE old.verdict <> excluded.verdict OR old.at <= $4 - interval '1 minute'",
    )
    .bind(jobs)
    .bind(offer.device)
    .bind(machine)
    .bind(now)
    .bind(stored(offer.memory_bytes)?)
    .bind(stored(offer.disk_bytes)?)
    .bind(verdicts)
    .execute(conn)
    .await?;
    Ok(())
}

type DecisionRow = (DeviceId, String, String, i64, i64, DateTime<Utc>);

/// The last decision of each machine for the job, the newest first.
pub(in crate::work::jobs) async fn decisions_of(
    conn: &mut PgConnection,
    job: JobId,
) -> Result<Vec<Decision>, Error> {
    let rows: Vec<DecisionRow> = sqlx::query_as(
        "SELECT device_id, machine, verdict, free_memory_bytes, free_disk_bytes, at \
         FROM jobs.decisions WHERE job_id = $1 ORDER BY at DESC, device_id",
    )
    .bind(job)
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|(device, machine, verdict, memory, disk, at)| {
            Ok(Decision {
                device,
                machine,
                verdict: Verdict::from_stored(&verdict).ok_or(Error::Unreadable)?,
                memory_bytes: count(memory)?,
                disk_bytes: count(disk)?,
                at,
            })
        })
        .collect()
}
