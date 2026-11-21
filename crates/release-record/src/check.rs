//! The check of a record and of its file, in the order of ROL-010: the signature with the
//! public key of the program, then the version, the target and the program in the record, then
//! the size and the SHA-256 of the file. Then whether the machine takes the version (ROL-012,
//! ROL-041).

use std::fmt::Write as _;

use sha2::{Digest, Sha256};
use updater_signature::{PublicKey, SignatureCheck};

use crate::{
    MAX_RECORD_BYTES, RECORD_FILE,
    error::Refusal,
    model::{File, Program, Record, Target},
    version::Version,
};

/// What the checker asked for: the version that it was told, its own target, and the program
/// that it replaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    pub version: Version,
    pub target: Target,
    pub program: Program,
}

/// A record whose signature and fields were checked, with the file of the program asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub record: Record,
    pub file: File,
}

/// Checks `bytes`, the record as it came, with `signature`, the text of its `.sig` file.
///
/// # Errors
///
/// `RecordSignature` when the signature does not hold for these bytes, for `expected.version`
/// and for the file name of a record. `RecordMismatch` when the bytes are longer than a record,
/// are not a record, break a rule of its fields, or name a different version or target, or no
/// file of `expected.program`.
pub fn check(
    bytes: &[u8],
    signature: &str,
    public: &PublicKey,
    expected: Expected,
) -> Result<Checked, Refusal> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(Refusal::RecordMismatch);
    }
    let version = expected.version.to_string();
    let verified = SignatureCheck::new(public, signature).and_then(|check| {
        let mut stream = check.stream()?;
        stream.update(bytes);
        stream.finish_named(&version, Some(RECORD_FILE))
    });
    verified.map_err(|_| Refusal::RecordSignature)?;
    let record: Record = serde_json::from_slice(bytes).map_err(|_| Refusal::RecordMismatch)?;
    let file = record
        .file_of(expected.program)
        .filter(|_| {
            record.is_valid()
                && record.version == expected.version
                && record.target == expected.target
        })
        .cloned()
        .ok_or(Refusal::RecordMismatch)?;
    Ok(Checked { record, file })
}

/// The check of one file against its entry in a checked record, as the bytes arrive. It reads
/// at most the size that the record gives.
pub struct FileCheck {
    expected: File,
    seen: u64,
    hash: Sha256,
}

impl FileCheck {
    #[must_use]
    pub fn new(file: &File) -> Self {
        Self {
            expected: file.clone(),
            seen: 0,
            hash: Sha256::new(),
        }
    }

    /// Feeds the next bytes of the file.
    ///
    /// # Errors
    ///
    /// `FileMismatch` as soon as more bytes came than the record gives.
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), Refusal> {
        let length = u64::try_from(chunk.len()).map_err(|_| Refusal::FileMismatch)?;
        self.seen = self.seen.checked_add(length).ok_or(Refusal::FileMismatch)?;
        if self.seen > self.expected.size {
            return Err(Refusal::FileMismatch);
        }
        self.hash.update(chunk);
        Ok(())
    }

    /// Ends the check.
    ///
    /// # Errors
    ///
    /// `FileMismatch` when the size or the SHA-256 is not that of the record.
    pub fn finish(self) -> Result<(), Refusal> {
        if self.seen == self.expected.size && to_hex(&self.hash.finalize()) == self.expected.sha256
        {
            Ok(())
        } else {
            Err(Refusal::FileMismatch)
        }
    }
}

/// `bytes` as hexadecimal digits in lower case.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            // Writing to a String cannot fail.
            let _ = write!(text, "{byte:02x}");
            text
        })
}

/// Why a machine takes a version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admitted {
    /// The version is higher than the installed one and the release can replace it.
    Update,
    /// The version is the previous version that the machine keeps (ROL-031).
    Rollback,
}

/// Whether a machine with `installed`, and `previous` kept, takes the version of `record`.
///
/// # Errors
///
/// `LowerVersion` when the version is not higher than `installed` and is not `previous`
/// (ROL-012); `NotReplaceable` when the record says that the release cannot replace
/// `installed` (ROL-041).
pub fn admits(
    record: &Record,
    installed: Version,
    previous: Option<Version>,
) -> Result<Admitted, Refusal> {
    if record.version > installed {
        return if record.replaces_from <= installed {
            Ok(Admitted::Update)
        } else {
            Err(Refusal::NotReplaceable)
        };
    }
    if Some(record.version) == previous && record.version < installed {
        Ok(Admitted::Rollback)
    } else {
        Err(Refusal::LowerVersion)
    }
}
