//! Why a machine or the hub refuses a release: the causes of K8 Section 8.1 that a check of the
//! record and its file gives. Each has a fixed code and holds no text of the record.

/// A refusal, with the code that a daemon reports in `last.cause`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// The signature of the record is not good, or not for this version and this file name
    /// (ROL-010).
    #[error("the signature of the release record is not good")]
    RecordSignature,
    /// The record cannot be read, breaks a rule, or names a different version, target or
    /// program (ROL-010, ROL-013).
    #[error("the release record is not the record of this version, target and program")]
    RecordMismatch,
    /// The size or the SHA-256 of the file is not that of the record (ROL-010).
    #[error("the file is not the file that the release record names")]
    FileMismatch,
    /// The version is not higher than the installed one, and it is not the previous version
    /// (ROL-012).
    #[error("the version is not higher than the installed version")]
    LowerVersion,
    /// The record says that the release cannot replace the installed version (ROL-041).
    #[error("the release cannot replace the installed version")]
    NotReplaceable,
}

impl Refusal {
    /// The code of K8 Section 8.1.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RecordSignature => "record_signature",
            Self::RecordMismatch => "record_mismatch",
            Self::FileMismatch => "file_mismatch",
            Self::LowerVersion => "lower_version",
            Self::NotReplaceable => "not_replaceable",
        }
    }
}
