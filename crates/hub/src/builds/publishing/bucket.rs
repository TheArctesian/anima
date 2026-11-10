//! The release bucket (BLD-060 to BLD-062, D-24, D-28): where its objects are, the presigned
//! upload URL of one object for a builder, and the signed requests of the hub's own reads and
//! writes. The R2 API token is read from its two files once, at start, and held only as
//! `Zeroizing`; the type has no `Debug`, and no error or log names a token, a signature or a
//! presigned URL (LOG-007).

use std::path::Path;

use chrono::{DateTime, TimeDelta, Utc};
use zeroize::Zeroizing;

use super::{
    signature::UpdaterKey,
    sigv4::{self, Key, Request},
};
use crate::{builds::model::UploadedFile, config::Releases};

/// How long a presigned upload URL works (BLD-060, D-28).
pub(super) const UPLOAD_URL_LIFE: TimeDelta = TimeDelta::minutes(15);

/// The `Cache-Control` of a versioned file (Section 3.10.4).
pub(super) const VERSIONED_CACHE: &str = "public, max-age=3600";

/// The `Cache-Control` of a stable name and of `latest.json` (BLD-066).
pub(super) const NO_CACHE: &str = "no-cache";

/// R2 takes any region name and documents `auto`.
const REGION: &str = "auto";
const SERVICE: &str = "s3";

/// The largest token file the hub reads.
const MAX_TOKEN_FILE_BYTES: usize = 1024;

/// Why the bucket cannot be opened at start. Names the setting, never a value (SEC-004).
#[derive(Debug, thiserror::Error)]
pub enum BucketError {
    #[error("the R2 access key ID file cannot be read ({0})")]
    AccessKeyId(std::io::ErrorKind),
    #[error("the R2 secret access key file cannot be read ({0})")]
    SecretAccessKey(std::io::ErrorKind),
    #[error("ANIMA_UPDATER_PUBLIC_KEY is not a minisign public key")]
    UpdaterKey,
}

/// One presigned upload: the URL, the headers the `PUT` must send as given, and its end. No
/// `Debug`: the URL is a credential for its object (LOG-007).
pub struct UploadUrl {
    pub name: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub expires_at: DateTime<Utc>,
}

/// A write of the hub's own: the object, its headers other than the signature's, and the body.
pub(super) struct Write<'a> {
    pub(super) object: &'a str,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: &'a [u8],
}

/// A request of the hub's own, signed: the method, the URL and every header to send.
pub(super) struct Signed {
    pub(super) method: &'static str,
    pub(super) url: String,
    pub(super) headers: Vec<(String, String)>,
}

/// The R2 bucket releases go to, with the key to sign for it and the updater key its files must
/// verify with. No `Debug`.
pub struct Bucket {
    /// `scheme://host[:port]`.
    endpoint: String,
    name: String,
    public_base: String,
    access_key_id: Zeroizing<String>,
    secret: Zeroizing<String>,
    updater_key: Option<UpdaterKey>,
}

/// The trimmed text of a token file of at most 1 KiB. The buffer holds the most the file may
/// have, so it never grows and leaves no copy behind.
fn token(path: &Path) -> std::io::Result<Zeroizing<String>> {
    use std::io::Read as _;
    let mut text = Zeroizing::new(String::with_capacity(MAX_TOKEN_FILE_BYTES + 1));
    std::fs::File::open(path)?
        .take(MAX_TOKEN_FILE_BYTES as u64)
        .read_to_string(&mut text)?;
    let trimmed = Zeroizing::new(text.trim().to_owned());
    if trimmed.is_empty() {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(trimmed)
}

impl Bucket {
    /// Reads the token files named in `releases` and checks the updater key (I/O at the edge).
    ///
    /// # Errors
    ///
    /// A token file that cannot be read or is empty, or an updater key that is not one.
    pub fn open(releases: &Releases) -> Result<Self, BucketError> {
        let access_key_id =
            token(&releases.access_key_id_file).map_err(|e| BucketError::AccessKeyId(e.kind()))?;
        let secret = token(&releases.secret_access_key_file)
            .map_err(|e| BucketError::SecretAccessKey(e.kind()))?;
        let updater_key = releases
            .updater_public_key
            .as_deref()
            .map(|text| UpdaterKey::parse(text).ok_or(BucketError::UpdaterKey))
            .transpose()?;
        Ok(Self {
            endpoint: releases.endpoint.clone(),
            name: releases.bucket.clone(),
            public_base: releases.public_base.clone(),
            access_key_id,
            secret,
            updater_key,
        })
    }

    pub(super) fn public_base(&self) -> &str {
        &self.public_base
    }

    pub(super) const fn updater_key(&self) -> Option<&UpdaterKey> {
        self.updater_key.as_ref()
    }

    /// The URL people read `object` at.
    pub(super) fn public_url(&self, object: &str) -> String {
        format!("{}/{}", self.public_base, sigv4::encode(object, true))
    }

    fn key(&self) -> Key<'_> {
        Key {
            id: &self.access_key_id,
            secret: &self.secret,
            region: REGION,
            service: SERVICE,
        }
    }

    fn host(&self) -> &str {
        self.endpoint
            .split_once("://")
            .map_or(self.endpoint.as_str(), |(_, host)| host)
    }

    /// The path of `object` in the S3 API: the bucket, then the key.
    fn path(&self, object: &str) -> String {
        format!("/{}/{object}", self.name)
    }

    /// A presigned `PUT` of `file` in `folder` that must carry exactly its size, the versioned
    /// `Cache-Control` and `content_type`, valid for 15 minutes from `now` (BLD-060, D-28). It
    /// names this one object.
    pub(super) fn upload_url(
        &self,
        folder: &str,
        (file, content_type): (&UploadedFile, &str),
        now: DateTime<Utc>,
    ) -> UploadUrl {
        let headers = vec![
            ("content-length".to_owned(), file.size.to_string()),
            ("cache-control".to_owned(), VERSIONED_CACHE.to_owned()),
            ("content-type".to_owned(), content_type.to_owned()),
        ];
        let path = self.path(&format!("{folder}/{}", file.name));
        let request = Request {
            method: "PUT",
            host: self.host(),
            path: &path,
            query: Vec::new(),
            headers: headers.clone(),
        };
        let life = u32::try_from(UPLOAD_URL_LIFE.num_seconds()).unwrap_or(u32::MAX);
        let query = sigv4::presign(&self.key(), request, now, life);
        UploadUrl {
            name: file.name.clone(),
            url: format!("{}{}?{query}", self.endpoint, sigv4::encode(&path, true)),
            headers,
            expires_at: now + UPLOAD_URL_LIFE,
        }
    }

    /// The hub's own request `method` of `write`, signed at `now`.
    fn signed(&self, method: &'static str, write: Write<'_>, now: DateTime<Utc>) -> Signed {
        let path = self.path(write.object);
        let request = Request {
            method,
            host: self.host(),
            path: &path,
            query: Vec::new(),
            headers: write.headers,
        };
        let payload = sigv4::sha256_hex(write.body);
        Signed {
            method,
            url: format!("{}{}", self.endpoint, sigv4::encode(&path, true)),
            headers: sigv4::signed_headers(&self.key(), request, &payload, now),
        }
    }

    /// The hub's own signed `PUT` of `write`.
    pub(super) fn put(&self, write: Write<'_>, now: DateTime<Utc>) -> Signed {
        self.signed("PUT", write, now)
    }

    /// A signed `GET` of `object` from the bucket itself, past any cache (BLD-064, D-28).
    pub(super) fn get(&self, object: &str, now: DateTime<Utc>) -> Signed {
        let write = Write {
            object,
            headers: Vec::new(),
            body: b"",
        };
        self.signed("GET", write, now)
    }

    /// A signed `DELETE` of `object` (BLD-070).
    pub(super) fn delete(&self, object: &str, now: DateTime<Utc>) -> Signed {
        let write = Write {
            object,
            headers: Vec::new(),
            body: b"",
        };
        self.signed("DELETE", write, now)
    }

    /// `CopyObject` of `from` to the stable name `to` in this bucket, with no caching, only
    /// while `from` still has the `ETag` the hub checked (BLD-066, D-28).
    pub(super) fn copy(&self, (from, etag): (&str, &str), to: &str, now: DateTime<Utc>) -> Signed {
        let source = sigv4::encode(&self.path(from), true);
        let headers = vec![
            ("x-amz-copy-source".to_owned(), source),
            ("x-amz-copy-source-if-match".to_owned(), etag.to_owned()),
            ("x-amz-metadata-directive".to_owned(), "REPLACE".to_owned()),
            ("cache-control".to_owned(), NO_CACHE.to_owned()),
        ];
        let write = Write {
            object: to,
            headers,
            body: b"",
        };
        self.put(write, now)
    }
}
