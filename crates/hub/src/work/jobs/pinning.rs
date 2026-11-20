//! The pins and the exclusions of the operator, and the reason of a placement (ROU-004,
//! ROU-006; R-259): a job or a project can name one machine, and then no different machine
//! ever gets the work; an excluded machine gets only work that names it.

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgConnection;

use super::{Asker, Error, Queue, ended, model::Job, placement, reasons, store, trail};
use crate::ids::{DeviceId, JobId, ProjectId};

/// Refuses a new job whose named or pinned machine can never run it, with the reason. A
/// machine that only has too little free memory or disk now is not refused: the job waits.
pub(super) async fn named_can_run(
    conn: &mut PgConnection,
    job: &Job,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let pins = store::pins(conn).await?;
    let Some(device) = placement::named(job, &pins) else {
        return Ok(());
    };
    let machine = store::machine_locked(conn, device).await?;
    let offer = store::offer_of(conn, device).await?;
    let known = machine.as_ref().zip(offer.as_ref());
    match placement::never_fits(job, known, now) {
        Some(why) => Err(Error::NoFit(reasons::unfit(job, why))),
        None => Ok(()),
    }
}

/// Pins `device` for each job of the project that names no machine. Returns false when there
/// is no such device.
///
/// # Errors
/// `Database` or `Events` when the change cannot be written; nothing is changed then.
pub async fn pin_project(
    queue: Queue<'_>,
    (project, device): (ProjectId, DeviceId),
    asker: Asker,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = async {
        if !store::pin(&mut tx, project, device).await? {
            return Ok(false);
        }
        let change = json!({ "pinnedForProject": project });
        trail::placement_set(&mut tx, (device, change), (asker, now)).await?;
        Ok(true)
    }
    .await;
    ended(tx, outcome).await
}

/// Removes the pin of the project. Returns false when the project had none.
///
/// # Errors
/// `Database` or `Events` when the change cannot be written; nothing is changed then.
pub async fn unpin_project(
    queue: Queue<'_>,
    project: ProjectId,
    asker: Asker,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = async {
        let Some(device) = store::unpin(&mut tx, project).await? else {
            return Ok(false);
        };
        let change = json!({ "unpinnedForProject": project });
        trail::placement_set(&mut tx, (device, change), (asker, now)).await?;
        Ok(true)
    }
    .await;
    ended(tx, outcome).await
}

/// Excludes the machine from work that names no machine, or takes the exclusion away. Returns
/// false for a machine that never asked this hub for work.
///
/// # Errors
/// `Database` or `Events` when the change cannot be written; nothing is changed then.
pub async fn exclude(
    queue: Queue<'_>,
    (device, excluded): (DeviceId, bool),
    asker: Asker,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = async {
        if !store::set_excluded(&mut tx, device, excluded).await? {
            return Ok(false);
        }
        let change = json!({ "excluded": excluded });
        trail::placement_set(&mut tx, (device, change), (asker, now)).await?;
        Ok(true)
    }
    .await;
    ended(tx, outcome).await
}

/// The reason of the placement of the job as one line, from the last decision of each machine
/// that asked; `None` when there is no such job. The line is data (MCP-008).
///
/// # Errors
/// `Database` when it cannot be read.
pub async fn placement_reason(queue: Queue<'_>, id: JobId) -> Result<Option<String>, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = async {
        let Some(job) = store::job_locked(&mut tx, id).await? else {
            return Ok(None);
        };
        let decisions = store::decisions_of(&mut tx, id).await?;
        Ok(Some(reasons::line(&job, &decisions)))
    }
    .await;
    ended(tx, outcome).await
}
