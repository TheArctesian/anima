//! The hub's updater signature of a release (R-116, BLD-045, BLD-046, UPD-002): signs the
//! BLAKE2b-512 hash of the updater file's checked bytes, with the version in the trusted
//! comment, checks the signature with the updater's own verifier on a second read of the same
//! object, and writes the `.sig` beside the file. Each try signs the same bytes: the trusted
//! comment's time is the time the job reached `uploading`, so a retry writes what an earlier try
//! wrote (D-25). A signature or key never reaches a log or a refusal (LOG-007).

use updater_signature::Signed;

use super::{
    bucket::{VERSIONED_CACHE, Write},
    feed,
    run::{Reach, Work},
    transfer::{self, Stop},
};
use crate::builds::model::UploadedFile;

/// The longest `.sig` text the hub reads back; one is a few hundred bytes.
const MAX_SIGNATURE_BYTES: usize = 4096;

/// The updater file as the hub checked it on its first read.
pub(super) struct Verified<'a> {
    pub(super) file: &'a UploadedFile,
    pub(super) etag: &'a str,
    /// The BLAKE2b-512 hash of the bytes the first read checked.
    pub(super) digest: [u8; 64],
}

/// The object of `name` in the job's folder.
fn object(work: &Work, name: &str) -> String {
    format!("{}/{}/{name}", work.target, work.version)
}

/// The refusal for a signature that cannot be made or does not hold; fixed text.
fn unsigned(name: &str, version: &str) -> Stop {
    Stop::Refused(format!(
        "The hub's signature of {name} does not verify for {version} with ANIMA_UPDATER_PUBLIC_KEY."
    ))
}

/// Signs the verified updater file for the job's version, then checks the signature on a second
/// read of the same object, which must still have the checked `ETag`, size and SHA-256: the
/// signature is over exactly the verified bytes (BLD-046, BLD-064). Returns the `.sig` text.
pub(super) async fn sign(
    reach: Reach<'_>,
    work: &Work,
    verified: &Verified<'_>,
) -> Result<String, Stop> {
    let version = work.version.to_string();
    let name = verified.file.name.as_str();
    let signed = Signed {
        timestamp: work.signed_at,
        file_name: name,
        version: &version,
    };
    let key = reach.bucket.updater_key();
    let text = key
        .sign(&verified.digest, &signed)
        .map_err(|_| unsigned(name, &version))?;
    let check = key.check(&text).map_err(|_| unsigned(name, &version))?;
    let mut stream = check.stream().map_err(|_| unsigned(name, &version))?;
    let get = reach.bucket.get(&object(work, name), reach.clock.now());
    let read = transfer::read(reach.http, get, verified.file, &mut |chunk| {
        stream.update(chunk);
    })
    .await?;
    if read.etag != verified.etag {
        return Err(Stop::Refused(format!(
            "{name} changed in the release bucket while the hub signed it."
        )));
    }
    stream
        .finish(&version)
        .map(|()| text)
        .map_err(|_| unsigned(name, &version))
}

/// Writes `text` as the `.sig` of the updater file `updater`. The same bytes on every try, so
/// a write whose answer was lost, or that a stop cut short, is put again whole (D-25).
pub(super) async fn write(
    reach: Reach<'_>,
    work: &Work,
    updater: &str,
    text: &str,
) -> Result<(), Stop> {
    let name = feed::signature_name(updater);
    let object = object(work, &name);
    let write = Write {
        object: &object,
        headers: vec![
            ("cache-control".to_owned(), VERSIONED_CACHE.to_owned()),
            (
                "content-type".to_owned(),
                "text/plain; charset=utf-8".to_owned(),
            ),
        ],
        body: text.as_bytes(),
    };
    let put = reach.bucket.put(write, reach.clock.now());
    transfer::send(reach.http, put, text.as_bytes().to_vec(), &name).await
}

/// The `.sig` text an earlier try wrote for `updater`, for the confirm of a sent feed (D-25).
/// Refused when it is missing, too long or not a signature by the hub's key.
pub(super) async fn written(reach: Reach<'_>, work: &Work, updater: &str) -> Result<String, Stop> {
    let name = feed::signature_name(updater);
    let get = reach.bucket.get(&object(work, &name), reach.clock.now());
    let bytes = transfer::read_small(reach.http, get, &name, MAX_SIGNATURE_BYTES).await?;
    let text =
        String::from_utf8(bytes).map_err(|_| unsigned(updater, &work.version.to_string()))?;
    reach
        .bucket
        .updater_key()
        .check(&text)
        .map_err(|_| unsigned(updater, &work.version.to_string()))?;
    Ok(text)
}
