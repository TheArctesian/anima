//! The publish of a built release to the bucket, in the publisher's order (BLD-002, BLD-064 to
//! BLD-069): the published versions, each file put and read back, the `.sig`, the stable name,
//! then each feed last. I/O only; the decisions are in `checks` and in the publisher's own
//! `feed` and `files`.

use reqwest::Client;
use serde_json::Value;

use super::{
    Built, BuiltError,
    checks::{bytes_of, check_built},
    refused,
};
use crate::{
    clock::Clock,
    fleet::builds::{
        model::UploadedFile,
        publishing::{
            bucket::Bucket,
            feed::{self, Expected},
            files,
            run::{self, Reach, Work},
            signature,
            transfer::{self, Stop},
        },
        version::Version,
    },
};

/// The most of a published `latest.json` that is read for its version.
const MAX_FEED_BYTES: usize = 64 * 1024;

/// What publishes a built release: the bucket, the HTTP client, the clock, and the updater
/// public key of the release commit.
pub struct BuiltPublisher<'a> {
    pub bucket: &'a Bucket,
    pub http: &'a Client,
    pub clock: &'a dyn Clock,
    pub public_key: &'a str,
}

impl BuiltPublisher<'_> {
    const fn reach(&self) -> Reach<'_> {
        Reach {
            bucket: self.bucket,
            http: self.http,
            clock: self.clock,
        }
    }

    /// The version `object`, a `latest.json`, gives in the bucket now; `None` when the bucket
    /// has no such object. Refused when `feed`, the feed of this release, is not the one the
    /// bucket holds for the same version.
    async fn published(
        &self,
        object: &str,
        (version, feed): (Version, &str),
    ) -> Result<Option<Version>, BuiltError> {
        let reach = self.reach();
        let probe = reach.bucket.head(object, reach.clock.now());
        if transfer::etag_now(reach.http, &probe).await?.is_none() {
            return Ok(None);
        }
        let get = reach.bucket.get(object, reach.clock.now());
        let held = transfer::read_small(reach.http, get, object, MAX_FEED_BYTES).await?;
        let unread = || {
            refused(format!(
                "{object} holds a feed whose version cannot be read."
            ))
        };
        let top: Value = serde_json::from_slice(&held).map_err(|_| unread())?;
        let published: Version = top
            .get("version")
            .and_then(Value::as_str)
            .and_then(|text| text.parse().ok())
            .ok_or_else(unread)?;
        if published == version && held != feed.as_bytes() {
            return Err(refused(format!(
                "{object} already holds another feed of {version}."
            )));
        }
        Ok(Some(published))
    }

    /// The highest version the feeds of the target give now (BLD-002, BLD-069).
    async fn floor(&self, built: &Built, feed: &str) -> Result<Option<Version>, BuiltError> {
        let mut floor = None;
        for folder in feed::platform_keys(built.target) {
            let object = format!("{folder}/latest.json");
            let published = self.published(&object, (built.version, feed)).await?;
            floor = floor.max(published);
        }
        Ok(floor)
    }

    /// Puts `file` at its versioned name unless the bucket holds it, then reads it back from
    /// the bucket itself and checks its size and SHA-256, and that people can read it
    /// (BLD-064). A name the bucket already holds with other bytes is refused: a version is
    /// never built twice (BLD-002). Returns the `ETag` of the checked object.
    async fn place(&self, built: &Built, file: &UploadedFile) -> Result<String, BuiltError> {
        let reach = self.reach();
        let folder = format!("{}/{}", built.target, built.version);
        let object = format!("{folder}/{}", file.name);
        let probe = reach.bucket.head(&object, reach.clock.now());
        let held = transfer::etag_now(reach.http, &probe).await?.is_some();
        if !held {
            let kind = files::content_type(built.target, built.version, &file.name)
                .ok_or_else(|| refused(format!("{} is not a file of this release.", file.name)))?;
            let bytes = bytes_of(built, &file.name).cloned().unwrap_or_default();
            let upload = reach
                .bucket
                .upload_url(&folder, (file, kind), reach.clock.now());
            transfer::upload(reach.http, upload, bytes).await?;
        }
        let get = reach.bucket.get(&object, reach.clock.now());
        let read = match transfer::read(reach.http, get, file, &mut |_| {}).await {
            Err(Stop::Refused(_)) if held => {
                return Err(refused(format!(
                    "{object} is already in the release bucket with other bytes."
                )));
            }
            other => other?,
        };
        transfer::readable(reach.http, &reach.bucket.public_url(&object), &file.name).await?;
        Ok(read.etag)
    }

    /// Publishes `built` in the publisher's order (BLD-064 to BLD-069): every check of
    /// [`check_built`], the published versions, each file put and read back, the `.sig`, the
    /// stable name by the checked `ETag`, then `latest.json` of each feed folder last. Returns
    /// the feed folders written. Each step can run again: a file the bucket holds with the same
    /// bytes is kept, and the same feed is written again.
    ///
    /// # Errors
    ///
    /// `Refused` when a check fails, with the feed as it was; `Unavailable` when the bucket
    /// does not answer.
    pub async fn publish(&self, built: &Built) -> Result<&'static [&'static str], BuiltError> {
        let reach = self.reach();
        let checked = check_built(built, self.public_key, reach.bucket.public_base())?;
        let feed = feed::signed(&checked.feed, &checked.signature).map_err(refused)?;
        let floor = self.floor(built, &feed).await?;
        let expected = Expected {
            target: built.target,
            version: built.version,
            public_base: reach.bucket.public_base(),
            files: &checked.files,
            floor,
        };
        feed::check(&checked.feed, &expected).map_err(refused)?;
        let (source, stable) = files::stable_copy(built.target, &checked.files).map_err(refused)?;
        let mut source_etag = String::new();
        for file in &checked.files {
            let etag = self.place(built, file).await?;
            if file.name == source {
                source_etag = etag;
            }
        }
        let work = Work {
            target: built.target,
            version: built.version,
            files: checked.files.clone(),
            feed: checked.feed.clone(),
            floor,
            feed_sent: false,
            signed_at: reach.clock.now(),
        };
        signature::write(reach, &work, &checked.updater, &checked.signature).await?;
        let from = format!("{}/{}/{source}", built.target, built.version);
        let copy = reach
            .bucket
            .copy((&from, &source_etag), stable, reach.clock.now());
        transfer::send(reach.http, copy, Vec::new(), stable).await?;
        let folders = feed::platform_keys(built.target);
        for folder in folders {
            run::write_feed(reach, &format!("{folder}/latest.json"), &feed).await?;
        }
        Ok(folders)
    }
}
