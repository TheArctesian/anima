//! One publish of one job, in the order of BLD-064 to BLD-066: check the feed and the file set,
//! read every file back from the bucket itself and check it, sign the updater file's checked
//! bytes and write its `.sig` (R-117), copy the stable name by the checked `ETag`, then write
//! `latest.json` with the signature last (UPD-008, D-28), and read the checked files' `ETag`s
//! again. Before each write the try checks it still holds the job (D-25). Each step may
//! run again after a stop: a read changes nothing and a write puts the same bytes. Any refusal
//! leaves `latest.json` as it was (BLD-070). Once a feed write was sent, a later try only
//! confirms the feed (D-25).

use chrono::{DateTime, TimeDelta, Utc};
use reqwest::Client;
use sqlx::PgPool;
use updater_signature::Prehash;

use super::{
    bucket::{Bucket, NO_CACHE, Write},
    feed::{self, Expected, SUMS},
    files,
    signature::{self, Verified},
    sigv4,
    transfer::{self, Stop},
};
use crate::{
    builds::{
        model::{Target, UploadedFile},
        store,
        version::Version,
    },
    clock::Clock,
    ids::BuildJobId,
};

/// What one publish ended with.
pub(super) enum Outcome {
    /// Published, with the feed folders written (one per platform key, BLD-067, BLD-068) and the checked files
    /// the bucket held other bytes for afterwards (D-28).
    Published(&'static [&'static str], Vec<String>),
    Refused(String),
    Unavailable(&'static str),
    /// A later try took the job, or the hold ran out; this try records nothing (D-25).
    TakenOver,
    /// The job's tries were spent before this one ran.
    Spent,
}

/// What one job brings to its publish.
pub(super) struct Work {
    pub(super) target: Target,
    pub(super) version: Version,
    /// The files the upload URLs were given for (D-26).
    pub(super) files: Vec<UploadedFile>,
    pub(super) feed: String,
    /// The highest version the target has published (D-27).
    pub(super) floor: Option<Version>,
    /// An earlier try sent a feed write for the job (D-25).
    pub(super) feed_sent: bool,
    /// The time in the signature's trusted comment: when the job reached `uploading`, the same
    /// on every try (R-117).
    pub(super) signed_at: DateTime<Utc>,
}

/// What a publish reaches: the bucket, the HTTP client, and the clock read for each signed
/// request (D-25).
#[derive(Clone, Copy)]
pub(super) struct Reach<'a> {
    pub(super) bucket: &'a Bucket,
    pub(super) http: &'a Client,
    pub(super) clock: &'a dyn Clock,
}

/// One try of the publisher on one job, as the fencing token of its writes (D-25).
#[derive(Clone, Copy)]
pub(super) struct Tried {
    pub(super) job: BuildJobId,
    pub(super) attempt: i32,
}

impl Tried {
    /// Stops unless this try still holds its job in `db` for as long as a write may take: the
    /// check just before a write.
    async fn holds(self, db: &PgPool, clock: &dyn Clock) -> Result<(), Stop> {
        let write = TimeDelta::seconds(transfer::WRITE_LIMIT.as_secs().cast_signed());
        match store::still_held(db, self.job, self.attempt, clock.now() + write).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(Stop::TakenOver),
            Err(_) => Err(Stop::Unavailable("database")),
        }
    }
}

/// Publishes `work` as try `tried`, checking its hold in `db` before each write.
pub(super) async fn publish(reach: Reach<'_>, db: &PgPool, tried: Tried, work: &Work) -> Outcome {
    match steps(reach, (db, tried), work).await {
        Ok((folders, changed)) => Outcome::Published(folders, changed),
        Err(Stop::Refused(reason)) => Outcome::Refused(reason),
        Err(Stop::Unavailable(fault)) => Outcome::Unavailable(fault),
        Err(Stop::TakenOver) => Outcome::TakenOver,
    }
}

/// What the first read of a job's files found: the bytes of `SHA256SUMS`, each file's `ETag` by
/// name, and the BLAKE2b-512 hash of the updater file's checked bytes.
struct FirstRead {
    sums: Vec<u8>,
    etags: Vec<(String, String)>,
    digest: [u8; 64],
}

/// Reads each file back from the bucket and checks it, hashing the updater file's bytes for
/// its signature as they are counted, then that people can read it (BLD-064, D-28).
async fn verify_files(reach: Reach<'_>, work: &Work, updater: &str) -> Result<FirstRead, Stop> {
    let (mut sums, mut etags) = (Vec::new(), Vec::new());
    let mut prehash = Prehash::new();
    for file in &work.files {
        let object = format!("{}/{}/{}", work.target, work.version, file.name);
        let get = reach.bucket.get(&object, reach.clock.now());
        let read = if file.name == updater {
            transfer::read(reach.http, get, file, &mut |chunk| prehash.update(chunk)).await?
        } else {
            transfer::read(reach.http, get, file, &mut |_| {}).await?
        };
        transfer::readable(reach.http, &reach.bucket.public_url(&object), &file.name).await?;
        if file.name == SUMS {
            sums = read.kept;
        }
        etags.push((file.name.clone(), read.etag));
    }
    Ok(FirstRead {
        sums,
        etags,
        digest: prehash.finish(),
    })
}

/// The `ETag` checked for `name`, empty when it was not checked.
fn etag_of<'a>(etags: &'a [(String, String)], name: &str) -> &'a str {
    etags
        .iter()
        .find(|(file, _)| file == name)
        .map_or("", |(_, etag)| etag.as_str())
}

async fn steps(
    reach: Reach<'_>,
    (db, tried): (&PgPool, Tried),
    work: &Work,
) -> Result<(&'static [&'static str], Vec<String>), Stop> {
    if work.feed_sent {
        return Ok((confirm(reach, work).await?, Vec::new()));
    }
    let expected = Expected {
        target: work.target,
        version: work.version,
        public_base: reach.bucket.public_base(),
        files: &work.files,
        floor: work.floor,
    };
    let checked = feed::check(&work.feed, &expected).map_err(Stop::Refused)?;
    let (source, stable) = files::stable_copy(work.target, &work.files).map_err(Stop::Refused)?;
    let first = verify_files(reach, work, &checked.updater).await?;
    files::check_sums(&first.sums, &work.files).map_err(Stop::Refused)?;
    let etags = first.etags;
    let updater = work
        .files
        .iter()
        .find(|file| file.name == checked.updater)
        .ok_or_else(|| Stop::Refused("The feed does not name a file of the job.".to_owned()))?;
    let verified = Verified {
        file: updater,
        etag: etag_of(&etags, &checked.updater),
        digest: first.digest,
    };
    let sig = signature::sign(reach, work, &verified).await?;
    tried.holds(db, reach.clock).await?;
    signature::write(reach, work, &checked.updater, &sig).await?;
    let feed = feed::signed(&work.feed, &sig).map_err(Stop::Refused)?;
    tried.holds(db, reach.clock).await?;
    let from = format!("{}/{}/{source}", work.target, work.version);
    let copy = reach
        .bucket
        .copy((&from, etag_of(&etags, &source)), stable, reach.clock.now());
    transfer::send(reach.http, copy, Vec::new(), stable).await?;
    let folders = feed::platform_keys(work.target);
    for folder in folders {
        tried.holds(db, reach.clock).await?;
        store::mark_feed_sent(db, tried.job, reach.clock.now())
            .await
            .map_err(|_| Stop::Unavailable("database"))?;
        write_feed(reach, &format!("{folder}/latest.json"), &feed).await?;
    }
    let watched = [checked.updater.as_str(), SUMS];
    Ok((folders, changed(reach, work, &etags, &watched).await))
}

/// Confirms the feed an earlier try sent (D-25): every feed folder must hold this job's feed,
/// signed with the `.sig` that try wrote before it (R-117). A folder that holds another refuses
/// the job, whose files are kept.
async fn confirm(reach: Reach<'_>, work: &Work) -> Result<&'static [&'static str], Stop> {
    let folders = feed::platform_keys(work.target);
    let updater = feed::updater_name(work.target, &work.files)
        .ok_or_else(|| Stop::Refused("The job has no single updater file.".to_owned()))?;
    let sig = signature::written(reach, work, updater).await?;
    let feed = feed::signed(&work.feed, &sig).map_err(Stop::Refused)?;
    for folder in folders {
        let object = format!("{folder}/latest.json");
        match feed_in(reach, &object, &feed).await {
            Err(Stop::Refused(_)) => {
                return Err(Stop::Refused(format!(
                    "{object} holds another feed than this job's; its files are kept."
                )));
            }
            other => other?,
        }
    }
    Ok(folders)
}

/// The `watched` files whose `ETag` in the bucket is no longer the one checked: a builder's URL
/// outlived the check, as when the hub's clock runs ahead (D-28). A file that cannot be read
/// again is logged by its fault word only.
async fn changed(
    reach: Reach<'_>,
    work: &Work,
    etags: &[(String, String)],
    watched: &[&str],
) -> Vec<String> {
    let mut changed = Vec::new();
    for name in watched {
        let object = format!("{}/{}/{name}", work.target, work.version);
        let head = reach.bucket.head(&object, reach.clock.now());
        match transfer::etag_now(reach.http, &head).await {
            Ok(now) if now.as_deref() == Some(etag_of(etags, name)) => {}
            Ok(_) => changed.push((*name).to_owned()),
            Err(_) => tracing::warn!(
                domain = "builds",
                fault = "bucket head",
                "publishing could not read a published file's ETag again"
            ),
        }
    }
    changed
}

/// Writes `feed` to `object`. When the bucket's answer is lost, reads the object back: the
/// write may have landed, and then the job is published (D-25).
async fn write_feed(reach: Reach<'_>, object: &str, feed: &str) -> Result<(), Stop> {
    let write = Write {
        object,
        headers: vec![
            ("cache-control".to_owned(), NO_CACHE.to_owned()),
            ("content-type".to_owned(), "application/json".to_owned()),
        ],
        body: feed.as_bytes(),
    };
    let put = reach.bucket.put(write, reach.clock.now());
    match transfer::send(reach.http, put, feed.as_bytes().to_vec(), object).await {
        Err(Stop::Unavailable(fault)) if feed_in(reach, object, feed).await.is_err() => {
            Err(Stop::Unavailable(fault))
        }
        Err(Stop::Unavailable(_)) => Ok(()),
        sent => sent,
    }
}

/// Checks that the bucket holds exactly `feed` at `object`: refused when it holds other bytes or
/// none.
async fn feed_in(reach: Reach<'_>, object: &str, feed: &str) -> Result<(), Stop> {
    let expected = UploadedFile {
        name: object.to_owned(),
        sha256: sigv4::sha256_hex(feed.as_bytes()),
        size: i64::try_from(feed.len()).unwrap_or(i64::MAX),
    };
    let get = reach.bucket.get(object, reach.clock.now());
    transfer::read(reach.http, get, &expected, &mut |_| {})
        .await
        .map(|_| ())
}
