//! Publishing (BLD-060 to BLD-070, D-24 to D-28, R-116): the presigned upload URLs a builder
//! gets for its job, and the publisher that takes each job whose files are in, checks them, signs
//! the updater file with the hub's updater key and writes the `.sig`, the stable name and
//! `latest.json`.
//!
//! The publisher runs in the upkeep sweep (D-25), never inside a request. It takes a job in a
//! short transaction and holds it, so no row lock lives across the network calls; it checks the
//! hold again before each write, and records the end in a second transaction only if its try is
//! still the job's latest. A hub that stops mid-publish leaves the job `uploading` with a hold
//! that runs out; the next sweep after it takes the job again and runs every step again. After
//! three tries the job fails, so every job ends `published` or `failed`. A job is taken only
//! once every upload URL of it expired, so its files cannot change after they are checked
//! (D-28).

mod bucket;
mod feed;
mod files;
mod run;
mod signature;
mod sigv4;
#[cfg(test)]
mod tests;
mod transfer;
mod tries;
mod updater_key;

use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::Client;
use sqlx::PgPool;

pub use bucket::{Bucket, BucketError, UploadUrl};
use run::{Outcome, Reach};
pub use updater_key::{UpdaterKey, UpdaterKeyError};

use super::{
    BuildQueue, Builder, Error, FieldError, JobUpdate, UploadedFile, jobs, queue, store, upload,
};
use crate::{clock::Clock, config::Releases, ids::BuildJobId};

/// How often the upkeep runs the publisher (D-25).
pub const PUBLISH_SWEEP: Duration = Duration::from_secs(10);

/// The longest one try may run, well inside its hold.
const RUN_LIMIT: Duration = Duration::from_mins(7);

// A try ends at least 3 minutes before its hold, so a write it sent near the end lands before a
// later try can take the job (D-25).
const _: () = assert!(RUN_LIMIT.as_secs() + 180 <= tries::HOLD.num_seconds().unsigned_abs());

/// The most jobs one sweep publishes.
const MAX_PER_SWEEP: usize = 4;

/// The reason of a job handed to a hub whose publishing is off (D-25).
const PUBLISHING_OFF: &str = "Publishing is off on this hub: no release bucket, or no usable updater key (the hub log says which).";

/// The release bucket of `releases` with the updater key opened from its files (R-116); `None`,
/// publishing off, when the updater key cannot be used. The warning names the check that failed,
/// never a value (LOG-007), and the hub still starts, as with no release bucket.
///
/// # Errors
///
/// A token file of the bucket that cannot be read or is empty.
pub fn open_bucket(releases: &Releases) -> Result<Option<Bucket>, BucketError> {
    match UpdaterKey::open(releases) {
        Ok(key) => Bucket::open(releases, key).map(Some),
        Err(error) => {
            tracing::warn!(domain = "builds", %error, "publishing is off: the updater key cannot be used");
            Ok(None)
        }
    }
}

/// A builder's request for the upload URLs of the files of a job it holds (BLD-060).
pub struct Grant<'a> {
    pub builder: &'a Builder,
    pub job: BuildJobId,
    pub files: &'a [UploadedFile],
}

/// Presigned `PUT` URLs, one for each file of `grant`, for an `uploading` job its builder holds;
/// extends the lease and the time the job's URLs end (BLD-060, BLD-061, D-28). Only the files a
/// job of its target uploads are granted, each with its content type. The files become the
/// job's grant (D-26): the `uploaded` report must name exactly them.
///
/// # Errors
///
/// `Conflict` when the job is not `uploading` or the hub publishes it already; `Validation`
/// (`files`) for a bad file list or a name the job does not upload; as `report` otherwise.
pub async fn upload_urls(
    db: &PgPool,
    bucket: &Bucket,
    grant: Grant<'_>,
    now: DateTime<Utc>,
) -> Result<Vec<UploadUrl>, Error> {
    let files = upload::files(grant.files).map_err(|e| Error::Validation(vec![e]))?;
    let mut tx = db.begin().await?;
    let next = jobs::extend_uploading(&mut tx, grant.builder, grant.job, now).await?;
    let kinds = files
        .iter()
        .map(|file| files::content_type(next.target, next.version, &file.name))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            let message = format!("A {} job uploads only its release files.", next.target);
            Error::Validation(vec![FieldError::new("files", message)])
        })?;
    store::replace_files(&mut tx, next.id, &files).await?;
    store::save(&mut tx, &next).await?;
    store::extend_upload_expiry(&mut tx, next.id, now + bucket::UPLOAD_URL_LIFE).await?;
    tx.commit().await?;
    let folder = format!("{}/{}", next.target, next.version);
    Ok(files
        .iter()
        .zip(kinds)
        .map(|(file, kind)| bucket.upload_url(&folder, (file, kind), now))
        .collect())
}

/// Publishes the jobs whose files are in, whose upload URLs expired and whose hold ended, at
/// most four, one after the other, reading `clock` at each step (D-25, D-28). Returns how many
/// ended a try.
///
/// # Errors
///
/// `Database` or `Events` when a take or an end cannot be written; the job is then taken again
/// after its hold.
pub async fn publish_ready(
    build_queue: BuildQueue<'_>,
    bucket: &Bucket,
    http: &Client,
    clock: &dyn Clock,
) -> Result<usize, Error> {
    let reach = Reach {
        bucket,
        http,
        clock,
    };
    let mut ended = 0;
    while ended < MAX_PER_SWEEP {
        let Some((tried, work)) = tries::take(build_queue.db, clock.now()).await? else {
            break;
        };
        let outcome = match work {
            Some(work) => {
                let publish = run::publish(reach, build_queue.db, tried, &work);
                tokio::time::timeout(RUN_LIMIT, publish)
                    .await
                    .unwrap_or(Outcome::Unavailable("time limit"))
            }
            None => Outcome::Spent,
        };
        tries::finish(build_queue, reach, tried, outcome).await?;
        ended += 1;
    }
    Ok(ended)
}

/// Fails every job handed to the publisher of a hub whose publishing is off, so none waits for
/// ever (D-25). Returns how many failed.
///
/// # Errors
///
/// `Database` or `Events` when the failures cannot be written.
pub async fn publishing_off(
    build_queue: BuildQueue<'_>,
    now: DateTime<Utc>,
) -> Result<usize, Error> {
    let mut tx = build_queue.db.begin().await?;
    let mut failed = Vec::new();
    for job in store::handed_over(&mut tx).await? {
        let next = queue::publish_failed(&job, PUBLISHING_OFF, now);
        store::save(&mut tx, &next).await?;
        jobs::record_failure(&mut tx, build_queue.bus, &next).await?;
        failed.push(next);
    }
    tx.commit().await?;
    let count = failed.len();
    for job in failed {
        build_queue.updates.publish(JobUpdate {
            job,
            lines: Vec::new(),
        });
    }
    Ok(count)
}
