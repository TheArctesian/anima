//! One publish of one job, in the order of BLD-064 to BLD-066: check the feed and the file set,
//! read every file back from the bucket itself and check it, copy the stable name by the
//! checked `ETag`, then write `latest.json` last (UPD-008, D-28). Before each write the try
//! checks it still holds the job (D-25). Each step may run again after a stop: a read changes
//! nothing and a write puts the same bytes. Any refusal leaves `latest.json` as it was
//! (BLD-070).

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

/// Whether macOS builds have Developer ID signing and notarization. Until OD-13 they do not, so
/// no macOS feed is written (BLD-068, R-108).
const DEVELOPER_ID: bool = false;

/// The reason of every publish while the hub has no updater public key (R-108).
pub(super) const NO_UPDATER_KEY: &str =
    "The hub has no updater public key (ANIMA_UPDATER_PUBLIC_KEY), so it publishes nothing.";

/// What one publish ended with.
pub(super) enum Outcome {
    /// Published, with the feed folders written (none for macOS, BLD-068).
    Published(&'static [&'static str]),
    Refused(String),
    Unavailable(&'static str),
    /// A later try took the job, or the hold ran out; this try records nothing (D-25).
    TakenOver,
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
    /// Stops unless this try still holds its job in `db` now: the check just before a write.
    async fn holds(self, db: &PgPool, clock: &dyn Clock) -> Result<(), Stop> {
        match store::still_held(db, self.job, self.attempt, clock.now()).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(Stop::TakenOver),
            Err(_) => Err(Stop::Unavailable("database")),
        }
    }
}

/// Publishes `work` as try `tried`, checking its hold in `db` before each write.
pub(super) async fn publish(reach: Reach<'_>, db: &PgPool, tried: Tried, work: &Work) -> Outcome {
    match steps(reach, (db, tried), work).await {
        Ok(folders) => Outcome::Published(folders),
        Err(Stop::Refused(reason)) => Outcome::Refused(reason),
        Err(Stop::Unavailable(fault)) => Outcome::Unavailable(fault),
        Err(Stop::TakenOver) => Outcome::TakenOver,
    }
}

/// Reads each file back from the bucket and checks it, the updater file's signature too, then
/// that people can read it (BLD-064, D-28). Returns the bytes of `SHA256SUMS` and the `ETag` of
/// `source`.
async fn verify_files(
    reach: Reach<'_>,
    work: &Work,
    checked: &feed::CheckedFeed,
    source: &str,
) -> Result<(Vec<u8>, String), Stop> {
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
    let (mut sums, mut etag) = (Vec::new(), String::new());
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
        if file.name == source {
            etag = read.etag;
        }
    }
    Ok((sums, etag))
}

async fn steps(
    reach: Reach<'_>,
    (db, tried): (&PgPool, Tried),
    work: &Work,
) -> Result<&'static [&'static str], Stop> {
    let expected = Expected {
        target: work.target,
        version: work.version,
        public_base: reach.bucket.public_base(),
        files: &work.files,
        floor: work.floor,
    };
    let checked = feed::check(&work.feed, &expected).map_err(Stop::Refused)?;
    let (source, stable) = files::stable_copy(work.target, &work.files).map_err(Stop::Refused)?;
    let (sums, etag) = verify_files(reach, work, &checked, &source).await?;
    files::check_sums(&sums, &work.files).map_err(Stop::Refused)?;
    tried.holds(db, reach.clock).await?;
    let from = format!("{}/{}/{source}", work.target, work.version);
    let copy = reach.bucket.copy((&from, &etag), stable, reach.clock.now());
    transfer::send(reach.http, copy, Vec::new(), stable).await?;
    let folders = feed::feed_folders(work.target, DEVELOPER_ID);
    for folder in folders {
        tried.holds(db, reach.clock).await?;
        write_feed(reach, &format!("{folder}/latest.json"), &work.feed).await?;
    }
    Ok(folders)
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
        Err(Stop::Unavailable(fault)) if !landed(reach, object, feed).await => {
            Err(Stop::Unavailable(fault))
        }
        Err(Stop::Unavailable(_)) => Ok(()),
        sent => sent,
    }
}

/// Whether the bucket holds exactly `feed` at `object`.
async fn landed(reach: Reach<'_>, object: &str, feed: &str) -> bool {
    let expected = UploadedFile {
        name: object.to_owned(),
        sha256: sigv4::sha256_hex(feed.as_bytes()),
        size: i64::try_from(feed.len()).unwrap_or(i64::MAX),
    };
    let get = reach.bucket.get(object, reach.clock.now());
    transfer::read(reach.http, get, &expected, None)
        .await
        .is_ok()
}
