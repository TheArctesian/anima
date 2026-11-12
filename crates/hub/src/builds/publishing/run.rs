//! One publish of one job, in the order of BLD-064 to BLD-066: check the feed and the file set,
//! read every file back from the bucket itself and check it, copy the stable name by the
//! checked `ETag`, then write `latest.json` last (UPD-008, D-28), and read the checked files'
//! `ETag`s again. Before each write the try checks it still holds the job (D-25). Each step may
//! run again after a stop: a read changes nothing and a write puts the same bytes. Any refusal
//! leaves `latest.json` as it was (BLD-070). Once a feed write was sent, a later try only
//! confirms the feed (D-25).

use chrono::TimeDelta;
use reqwest::Client;
use sqlx::PgPool;

use super::{
    bucket::{Bucket, NO_CACHE, Write},
    feed::{self, Expected, SUMS},
    files, signature, sigv4,
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

/// The reason of every publish while the hub has no updater public key (R-108).
pub(super) const NO_UPDATER_KEY: &str =
    "The hub has no updater public key (ANIMA_UPDATER_PUBLIC_KEY), so it publishes nothing.";

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

/// Reads each file back from the bucket and checks it, the updater file's signature too, then
/// that people can read it (BLD-064, D-28). Returns the bytes of `SHA256SUMS` and each file's
/// `ETag`, by name.
async fn verify_files(
    reach: Reach<'_>,
    work: &Work,
    checked: &feed::CheckedFeed,
) -> Result<(Vec<u8>, Vec<(String, String)>), Stop> {
    let key = reach
        .bucket
        .updater_key()
        .ok_or_else(|| Stop::Refused(NO_UPDATER_KEY.to_owned()))?;
    let unsigned = || {
        Stop::Refused(format!(
            "The signature of {} does not verify for {} with the hub's updater key.",
            checked.updater, work.version
        ))
    };
    let signed = signature::signature_of(&checked.signature).ok_or_else(unsigned)?;
    let (mut sums, mut etags) = (Vec::new(), Vec::new());
    for file in &work.files {
        let verifier = if file.name == checked.updater {
            Some(key.verifier(&signed).ok_or_else(unsigned)?)
        } else {
            None
        };
        let object = format!("{}/{}/{}", work.target, work.version, file.name);
        let get = reach.bucket.get(&object, reach.clock.now());
        let read = transfer::read(reach.http, get, file, verifier).await?;
        let version = work.version.to_string();
        if read
            .verifier
            .is_some_and(|v| !signature::holds(v, &signed, &version))
        {
            return Err(unsigned());
        }
        transfer::readable(reach.http, &reach.bucket.public_url(&object), &file.name).await?;
        if file.name == SUMS {
            sums = read.kept;
        }
        etags.push((file.name.clone(), read.etag));
    }
    Ok((sums, etags))
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
    let (sums, etags) = verify_files(reach, work, &checked).await?;
    files::check_sums(&sums, &work.files).map_err(Stop::Refused)?;
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
        write_feed(reach, &format!("{folder}/latest.json"), &work.feed).await?;
    }
    let watched = [checked.updater.as_str(), SUMS];
    Ok((folders, changed(reach, work, &etags, &watched).await))
}

/// Confirms the feed an earlier try sent (D-25): every feed folder must hold this job's feed. A
/// folder that holds another refuses the job, whose files are kept.
async fn confirm(reach: Reach<'_>, work: &Work) -> Result<&'static [&'static str], Stop> {
    let folders = feed::platform_keys(work.target);
    for folder in folders {
        let object = format!("{folder}/latest.json");
        match feed_in(reach, &object, &work.feed).await {
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
    transfer::read(reach.http, get, &expected, None)
        .await
        .map(|_| ())
}
