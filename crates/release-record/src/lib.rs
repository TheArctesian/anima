//! The release record of one target (ROL-003 to ROL-005, ROL-010 to ROL-013, ROL-036, ROL-041;
//! task U2 of the update plan): the signed list of the program files of one release for one
//! target. The hub checks it before it offers a release (ROL-007), and the part of a machine
//! that replaces a program checks it again by itself (ROL-010).
//!
//! The record is JSON. Its signature is the updater signature (one key, OD-ROL-2) over the
//! bytes of the record, with the version and the file name [`RECORD_FILE`] in its trusted
//! comment (ROL-004). The bytes that were signed are the bytes that are read: no form of the
//! record is made again before its check.
//!
//! Pure: no file, no clock, no network, no log. The time of a signature is a parameter.

mod check;
mod error;
mod model;
mod sign;
mod version;

#[cfg(test)]
mod tests;

pub use check::{Admitted, Checked, Expected, FileCheck, admits, check};
pub use error::Refusal;
pub use model::{Database, FORMAT, File, Program, Record, Target};
pub use sign::{SignError, sign};
pub use version::{Version, VersionError};

/// The name of the record in `<target>/<version>/` of the release host (ROL-005), and the
/// file name in the trusted comment of its signature.
pub const RECORD_FILE: &str = "release-record.json";

/// The name of the signature of the record beside it.
pub const SIGNATURE_FILE: &str = "release-record.json.sig";

/// The largest record that is read: 64 KiB (K8 Section 8.3).
pub const MAX_RECORD_BYTES: usize = 64 * 1024;

/// The largest program file that a record can name: 256 MiB (K8 Section 8.3).
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
