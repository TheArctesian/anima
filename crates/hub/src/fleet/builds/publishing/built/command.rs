//! The `anima-hub release` commands of the release workflow (R-179): sign, check and publish
//! the files of one target that the jobs of `.github/workflows/release.yml` built. The
//! settings have the names the hub reads (K7). A key comes from a file, never from an
//! argument, and no key, password or token is in a result or an error (LOG-007). The binary
//! prints the result.

use std::{collections::BTreeMap, path::Path};

use reqwest::Client;
use zeroize::Zeroizing;

use super::{Built, BuiltError, BuiltFile, BuiltPublisher, check_built, sign_built};
use crate::{
    clock::SystemClock,
    config,
    fleet::builds::{
        model::Target,
        publishing::{
            bucket::{Bucket, BucketError},
            updater_key::{UpdaterKey, UpdaterKeyError},
        },
        version::Version,
    },
};

const PUBLIC_KEY: &str = "ANIMA_UPDATER_PUBLIC_KEY";
const KEY_FILE: &str = "ANIMA_UPDATER_KEY_FILE";
const KEY_PASSWORD_FILE: &str = "ANIMA_UPDATER_KEY_PASSWORD_FILE";
const PUBLIC_BASE: &str = "ANIMA_RELEASES_PUBLIC_BASE";
const ENDPOINT: &str = "ANIMA_RELEASES_ENDPOINT";
/// The time in the signature's trusted comment, in seconds since 1970: the commit time, so a
/// second run signs the same text.
const SIGNED_AT: &str = "SOURCE_DATE_EPOCH";

/// Why a release command stopped. Names a setting, a file kind or a check, never a value.
#[derive(Debug, thiserror::Error)]
pub enum ReleaseError {
    #[error(
        "usage: anima-hub release <sign | check | publish | record> \
         <linux-x86_64 | darwin-universal | windows-x86_64> <version> <folder>"
    )]
    Usage,
    #[error("{0} is not set or is not valid")]
    Setting(&'static str),
    #[error("the folder of the release files cannot be read ({0})")]
    Folder(std::io::ErrorKind),
    #[error("the updater key cannot be used: {0}")]
    Key(UpdaterKeyError),
    #[error("{0}")]
    Built(#[from] BuiltError),
    #[error("{0}")]
    Record(#[from] super::record::RecordError),
    #[error("{0}")]
    Bucket(#[from] BucketError),
    #[error("invalid configuration")]
    Config(#[from] config::Error),
}

pub(super) fn setting<'a>(
    env: &'a BTreeMap<String, String>,
    name: &'static str,
) -> Result<&'a str, ReleaseError> {
    env.get(name)
        .map(String::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or(ReleaseError::Setting(name))
}

/// Each regular file of `folder` with its bytes. A folder, a link or a name that is not
/// Unicode in it is refused: a release folder holds only the files of one target.
pub(super) fn files_of(folder: &Path) -> Result<Vec<BuiltFile>, ReleaseError> {
    let fault = |error: std::io::Error| ReleaseError::Folder(error.kind());
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(fault)? {
        let entry = entry.map_err(fault)?;
        let name = entry.file_name().into_string();
        let plain = entry.file_type().map_err(fault)?.is_file();
        let (Ok(name), true) = (name, plain) else {
            return Err(ReleaseError::Folder(std::io::ErrorKind::InvalidData));
        };
        let bytes = std::fs::read(entry.path()).map_err(fault)?;
        files.push(BuiltFile {
            name,
            bytes: bytes.into(),
        });
    }
    Ok(files)
}

/// The updater key of `ANIMA_UPDATER_KEY_FILE`, which must have the public key of
/// `ANIMA_UPDATER_PUBLIC_KEY`, and the time for the trusted comment.
pub(super) fn open_key(env: &BTreeMap<String, String>) -> Result<(UpdaterKey, u64), ReleaseError> {
    let secret = |name: &'static str| {
        std::fs::read(setting(env, name)?)
            .map(Zeroizing::new)
            .map_err(|_| ReleaseError::Setting(name))
    };
    let password = match env.get(KEY_PASSWORD_FILE) {
        Some(path) if !path.is_empty() => secret(KEY_PASSWORD_FILE)?,
        _ => Zeroizing::new(Vec::new()),
    };
    let key = UpdaterKey::from_parts(&secret(KEY_FILE)?, &password, setting(env, PUBLIC_KEY)?)
        .map_err(ReleaseError::Key)?;
    let signed_at = setting(env, SIGNED_AT)?
        .parse()
        .map_err(|_| ReleaseError::Setting(SIGNED_AT))?;
    Ok((key, signed_at))
}

/// Signs the updater file in `folder` with the key of `ANIMA_UPDATER_KEY_FILE` and writes its
/// `.sig` beside it.
fn sign(
    env: &BTreeMap<String, String>,
    built: &Built,
    folder: &Path,
) -> Result<String, ReleaseError> {
    let (key, signed_at) = open_key(env)?;
    let (name, text) = sign_built(&key, built, signed_at)?;
    std::fs::write(folder.join(&name), text).map_err(|error| ReleaseError::Folder(error.kind()))?;
    Ok(format!("Signed: {name} is written."))
}

/// Runs each check of a publish that needs no bucket.
fn check(env: &BTreeMap<String, String>, built: &Built) -> Result<String, ReleaseError> {
    let base = env
        .get(PUBLIC_BASE)
        .map_or("https://releases.anima.ing", String::as_str);
    let checked = check_built(built, setting(env, PUBLIC_KEY)?, base)?;
    Ok(format!(
        "Checked: {} {} with the updater file {}. Nothing is published.",
        built.target,
        built.version,
        checked.updater()
    ))
}

/// Publishes to the bucket of `ANIMA_RELEASES_ENDPOINT`, the feed last.
async fn publish(
    env: &BTreeMap<String, String>,
    built: &Built,
    http: &Client,
) -> Result<String, ReleaseError> {
    let releases = config::releases_from_env(env)?.ok_or(ReleaseError::Setting(ENDPOINT))?;
    let public_key = releases
        .updater_public_key
        .as_deref()
        .ok_or(ReleaseError::Setting(PUBLIC_KEY))?;
    let bucket = Bucket::without_key(&releases)?;
    let publisher = BuiltPublisher {
        bucket: &bucket,
        http,
        clock: &SystemClock,
        public_key,
    };
    let folders = publisher.publish(built).await?;
    Ok(format!(
        "Published: {} {}. Feeds written: {}.",
        built.target,
        built.version,
        folders.join(", ")
    ))
}

/// Runs `anima-hub release <verb> <target> <version> <folder>` with the settings in `env`;
/// `http` reaches the bucket for `publish`. Returns the line to print.
///
/// # Errors
///
/// The usage, a setting or file that cannot be used, or the check that refused the release.
pub async fn release_command(
    env: &BTreeMap<String, String>,
    args: &[&str],
    http: &Client,
) -> Result<String, ReleaseError> {
    let [verb, target, version, folder] = args else {
        return Err(ReleaseError::Usage);
    };
    let target = Target::ALL
        .into_iter()
        .find(|known| known.as_str() == *target)
        .ok_or(ReleaseError::Usage)?;
    let version: Version = version.parse().map_err(|_| ReleaseError::Usage)?;
    let folder = Path::new(folder);
    if *verb == "record" {
        return super::record::record(env, target, version, folder);
    }
    let built = Built {
        target,
        version,
        files: files_of(folder)?,
    };
    match *verb {
        "sign" => sign(env, &built, folder),
        "check" => check(env, &built),
        "publish" => publish(env, &built, http).await,
        _ => Err(ReleaseError::Usage),
    }
}
