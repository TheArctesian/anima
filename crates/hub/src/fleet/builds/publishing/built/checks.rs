//! The checks of a built release that need only its bytes (BLD-045, BLD-046, BLD-061,
//! BLD-065, UPD-010). The folder the files came from must hold `SHA256SUMS`, which the workflow
//! writes from the digests its build jobs reported, so a file another step changed is refused.
//! The signature is checked with the public key the release commit names, never with a key
//! the files brought. Pure.

use bytes::Bytes;
use serde_json::{Map, Value, json};
use updater_signature::{Prehash, PublicKey, SignatureCheck, Signed};

use super::{Built, BuiltError, refused};
use crate::fleet::builds::{
    model::UploadedFile,
    publishing::{
        feed::{self, Expected, SUMS},
        files, sigv4,
        updater_key::UpdaterKey,
    },
};

/// A built release that passed every check that needs no bucket.
pub struct CheckedBuilt {
    /// Each file but the `.sig`, as the bucket must hold it.
    pub(super) files: Vec<UploadedFile>,
    pub(super) updater: String,
    pub(super) signature: String,
    /// The feed without the signature, as [`feed::check`] takes it.
    pub(super) feed: String,
}

impl CheckedBuilt {
    /// The updater file the feed names.
    #[must_use]
    pub fn updater(&self) -> &str {
        &self.updater
    }
}

/// The files of `built` but the `.sig` of its updater file, each with its size and SHA-256,
/// and the one updater file among them.
fn measured(built: &Built) -> Result<(Vec<UploadedFile>, String), BuiltError> {
    let mut files: Vec<UploadedFile> = built
        .files
        .iter()
        .map(|file| UploadedFile {
            name: file.name.clone(),
            sha256: sigv4::sha256_hex(&file.bytes),
            size: i64::try_from(file.bytes.len()).unwrap_or(i64::MAX),
        })
        .collect();
    let updater = feed::updater_name(built.target, &files)
        .ok_or_else(|| {
            refused(format!(
                "A {} release needs one updater file.",
                built.target
            ))
        })?
        .to_owned();
    let signature = feed::signature_name(&updater);
    files.retain(|file| file.name != signature);
    Ok((files, updater))
}

pub(super) fn bytes_of<'a>(built: &'a Built, name: &str) -> Option<&'a Bytes> {
    let file = built.files.iter().find(|file| file.name == name)?;
    Some(&file.bytes)
}

/// Signs the updater file of `built` for its version with `key`, and checks the signature with
/// the updater's own verifier before it gives it back (BLD-045, BLD-046). `signed_at` is the
/// time in the trusted comment, so the same commit time gives the same text. Returns the name
/// of the `.sig` file and its text.
///
/// # Errors
///
/// `Refused` when `built` has no single updater file or the signature does not hold.
pub fn sign_built(
    key: &UpdaterKey,
    built: &Built,
    signed_at: u64,
) -> Result<(String, String), BuiltError> {
    let (_, updater) = measured(built)?;
    let version = built.version.to_string();
    let unsigned = || refused(format!("{updater} cannot be signed for {version}."));
    let bytes = bytes_of(built, &updater).ok_or_else(unsigned)?;
    let mut prehash = Prehash::new();
    prehash.update(bytes);
    let signed = Signed {
        timestamp: signed_at,
        file_name: &updater,
        version: &version,
    };
    let text = key
        .sign(&prehash.finish(), &signed)
        .map_err(|_| unsigned())?;
    let check = key.check(&text).map_err(|_| unsigned())?;
    let mut stream = check.stream().map_err(|_| unsigned())?;
    stream.update(bytes);
    stream.finish(&version).map_err(|_| unsigned())?;
    Ok((feed::signature_name(&updater), text))
}

/// The feed of `updater` with no signature: the version and one URL for each platform key.
fn unsigned_feed(built: &Built, public_base: &str, updater: &str) -> String {
    let url = format!("{public_base}/{}/{}/{updater}", built.target, built.version);
    let platforms: Map<String, Value> = feed::platform_keys(built.target)
        .iter()
        .map(|key| ((*key).to_owned(), json!({ "url": url })))
        .collect();
    json!({ "version": built.version.to_string(), "platforms": platforms }).to_string()
}

/// Whether `signature` is the updater signature of `bytes` for `version` by `public_key`, the
/// `plugins.updater.pubkey` text (UPD-002, `requireSignedVersion`).
fn verifies(public_key: &str, signature: &str, (bytes, version): (&[u8], &str)) -> bool {
    let Ok(public) = PublicKey::from_key_file(public_key) else {
        return false;
    };
    let Ok(check) = SignatureCheck::new(&public, signature) else {
        return false;
    };
    let Ok(mut stream) = check.stream() else {
        return false;
    };
    stream.update(bytes);
    stream.finish(version).is_ok()
}

/// Checks `built` with nothing but its bytes (BLD-046, BLD-061, BLD-065, UPD-010): only the
/// files a release of the target has, `SHA256SUMS` listing each of them with its digest, one
/// updater file whose `.sig` verifies for the version with `public_key`, and the feed the
/// publisher's own check passes under `public_base`.
///
/// # Errors
///
/// `Refused` with the first check that failed.
pub fn check_built(
    built: &Built,
    public_key: &str,
    public_base: &str,
) -> Result<CheckedBuilt, BuiltError> {
    let (files, updater) = measured(built)?;
    let signature_name = feed::signature_name(&updater);
    for file in &built.files {
        let known = file.name == signature_name
            || files::content_type(built.target, built.version, &file.name).is_some();
        if !known {
            return Err(refused(format!(
                "{} is not a file of a {} release of {}.",
                file.name, built.target, built.version
            )));
        }
    }
    files::stable_copy(built.target, &files).map_err(refused)?;
    let sums = bytes_of(built, SUMS).map_or(&[][..], |bytes| &bytes[..]);
    files::check_sums(sums, &files).map_err(refused)?;
    let unverified = || {
        refused(format!(
            "The signature of {updater} does not verify for {} with the updater public key.",
            built.version
        ))
    };
    let signature = bytes_of(built, &signature_name)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .ok_or_else(unverified)?;
    let updater_bytes = bytes_of(built, &updater).ok_or_else(unverified)?;
    let version = built.version.to_string();
    if !verifies(public_key, signature, (updater_bytes, &version)) {
        return Err(unverified());
    }
    let feed = unsigned_feed(built, public_base, &updater);
    let expected = Expected {
        target: built.target,
        version: built.version,
        public_base,
        files: &files,
        floor: None,
    };
    feed::check(&feed, &expected).map_err(refused)?;
    Ok(CheckedBuilt {
        files,
        updater,
        signature: signature.to_owned(),
        feed,
    })
}
