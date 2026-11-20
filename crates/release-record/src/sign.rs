//! The bytes of a record and their signature, for the job of a release that holds the key
//! (R-173).

use updater_signature::{Prehash, SecretKey, Signed};

use crate::{RECORD_FILE, model::Record};

/// Why a record was not signed. Holds no key and no text of the record (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SignError {
    #[error("the release record breaks a rule of its fields")]
    Invalid,
    #[error("the release record could not be signed")]
    Signature,
}

/// The bytes of `record` and their signature (the text of the `.sig` file), with `timestamp`
/// (seconds since the Unix epoch) in the trusted comment. The bytes end with a line break.
///
/// # Errors
///
/// `Invalid` when the record breaks a rule of [`Record::is_valid`]; `Signature` when the key
/// cannot sign.
pub fn sign(
    record: &Record,
    key: &SecretKey,
    timestamp: u64,
) -> Result<(Vec<u8>, String), SignError> {
    if !record.is_valid() {
        return Err(SignError::Invalid);
    }
    let mut bytes = serde_json::to_vec_pretty(record).map_err(|_| SignError::Invalid)?;
    bytes.push(b'\n');
    let mut hash = Prehash::new();
    hash.update(&bytes);
    let version = record.version.to_string();
    let signed = Signed {
        timestamp,
        file_name: RECORD_FILE,
        version: &version,
    };
    let signature = key
        .sign(&hash.finish(), &signed)
        .map_err(|_| SignError::Signature)?;
    Ok((bytes, signature))
}
