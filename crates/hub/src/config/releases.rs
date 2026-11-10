//! Where releases are published (R-107, R-108; BLD-060 to BLD-066): the R2 S3 endpoint, the
//! bucket, its public address, the files that hold the R2 API token and the updater public key.
//! Publishing is off when the endpoint is unset or empty. Only paths are read here; the token
//! files are opened by `builds` at start (SEC-004, BLD-062).

use std::{collections::BTreeMap, path::PathBuf};

use super::{Error, origin, value};

const ENDPOINT: &str = "ANIMA_RELEASES_ENDPOINT";
const BUCKET: &str = "ANIMA_RELEASES_BUCKET";
const PUBLIC_BASE: &str = "ANIMA_RELEASES_PUBLIC_BASE";
const ACCESS_KEY_ID_FILE: &str = "ANIMA_RELEASES_ACCESS_KEY_ID_FILE";
const SECRET_ACCESS_KEY_FILE: &str = "ANIMA_RELEASES_SECRET_ACCESS_KEY_FILE";
const UPDATER_PUBLIC_KEY: &str = "ANIMA_UPDATER_PUBLIC_KEY";

/// The release bucket's settings. No secret: the token is in the files named here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Releases {
    /// The S3 API origin, `https://<account>.r2.cloudflarestorage.com`; `http` only for a
    /// loopback host, so tests can point at a local fake.
    pub endpoint: String,
    pub bucket: String,
    /// The origin people and the hub read published files from, `https://releases.anima.ing`.
    pub public_base: String,
    pub access_key_id_file: PathBuf,
    pub secret_access_key_file: PathBuf,
    /// `plugins.updater.pubkey` of `tauri.conf.json`; the two must be the same. `None` when
    /// unset or empty: every publish then fails with a fixed reason (R-108).
    pub updater_public_key: Option<String>,
}

/// An R2 bucket name: 3 to 63 lower-case letters, digits and `-`, a letter or digit at each end.
fn bucket_name(text: &str) -> bool {
    let bytes = text.as_bytes();
    let inner = |b: &u8| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-';
    let edge = |b: Option<&u8>| b.is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    (3..=63).contains(&bytes.len())
        && bytes.iter().all(inner)
        && edge(bytes.first())
        && edge(bytes.last())
}

/// The release settings in `env`; `None` when `ANIMA_RELEASES_ENDPOINT` is unset or empty.
pub(super) fn from_env(env: &BTreeMap<String, String>) -> Result<Option<Releases>, Error> {
    let Some(endpoint) = env.get(ENDPOINT).filter(|text| !text.is_empty()) else {
        return Ok(None);
    };
    let bucket = value(env, BUCKET, "anima-releases")?;
    if !bucket_name(bucket) {
        return Err(Error::Bucket {
            variable: BUCKET,
            value: bucket.to_owned(),
        });
    }
    Ok(Some(Releases {
        endpoint: origin::canonical(ENDPOINT, endpoint)?,
        bucket: bucket.to_owned(),
        public_base: origin::canonical(
            PUBLIC_BASE,
            value(env, PUBLIC_BASE, "https://releases.anima.ing")?,
        )?,
        access_key_id_file: value(env, ACCESS_KEY_ID_FILE, "/run/secrets/r2_access_key_id")?.into(),
        secret_access_key_file: value(
            env,
            SECRET_ACCESS_KEY_FILE,
            "/run/secrets/r2_secret_access_key",
        )?
        .into(),
        updater_public_key: env
            .get(UPDATER_PUBLIC_KEY)
            .filter(|text| !text.trim().is_empty())
            .cloned(),
    }))
}
