//! The causes of a refusal of a model item (Section 5, with those of K10 Section 5), and the
//! causes of a failure after `preparing`. Each is a fixed code; no text of a runtime or a file
//! goes into a report (Section 3).

/// Why the machine refuses or fails a model item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cause {
    // Refusals of K10 Section 5 that a model item can get.
    TypeUnknown,
    KindNotPermitted,
    MemberUnknown,
    SecretUnknown,
    SwitchOff,
    Policy,
    // Refusals of Section 5.
    FlagOwned,
    RuntimeNotConfigured,
    RuntimeNotInstalled,
    RuntimeAdvisory,
    SourceNotPermitted,
    CopyUnknown,
    CopyNotReady,
    RemoteCode,
    DiskShare,
    MemoryShare,
    FileType,
    GpuUnknown,
    RuntimeTooOld,
    ActorNotPermitted,
    RevisionTooNew,
    // Failures after `preparing`.
    Checksum,
    Size,
    SourceFailed,
    Listener,
    IsolationFailed,
    LimitMemory,
    LoadCrash,
    HashMismatch,
    StartFailed,
    ProbeFailed,
    LimitDisk,
    ProgramMismatch,
    /// An archive of a runtime breaks a rule of the unpack (R-NEW-archiverejected).
    ArchiveRejected,
}

impl Cause {
    /// The cause as the report spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeUnknown => "type_unknown",
            Self::KindNotPermitted => "kind_not_permitted",
            Self::MemberUnknown => "member_unknown",
            Self::SecretUnknown => "secret_unknown",
            Self::SwitchOff => "switch_off",
            Self::Policy => "policy",
            Self::FlagOwned => "flag_owned",
            Self::RuntimeNotConfigured => "runtime_not_configured",
            Self::RuntimeNotInstalled => "runtime_not_installed",
            Self::RuntimeAdvisory => "runtime_advisory",
            Self::SourceNotPermitted => "source_not_permitted",
            Self::CopyUnknown => "copy_unknown",
            Self::CopyNotReady => "copy_not_ready",
            Self::RemoteCode => "remote_code",
            Self::DiskShare => "disk_share",
            Self::MemoryShare => "memory_share",
            Self::FileType => "file_type",
            Self::GpuUnknown => "gpu_unknown",
            Self::RuntimeTooOld => "runtime_too_old",
            Self::ActorNotPermitted => "actor_not_permitted",
            Self::RevisionTooNew => "revision_too_new",
            Self::Checksum => "checksum",
            Self::Size => "size",
            Self::SourceFailed => "source_failed",
            Self::Listener => "listener",
            Self::IsolationFailed => "isolation_failed",
            Self::LimitMemory => "limit_memory",
            Self::LoadCrash => "load_crash",
            Self::HashMismatch => "hash_mismatch",
            Self::StartFailed => "start_failed",
            Self::ProbeFailed => "probe_failed",
            Self::LimitDisk => "limit_disk",
            Self::ProgramMismatch => "program_mismatch",
            Self::ArchiveRejected => "archive_rejected",
        }
    }

    /// Whether the machine reports the cause with the state `refused` (before `preparing`),
    /// not `failed`.
    #[must_use]
    pub const fn is_refusal(self) -> bool {
        !matches!(
            self,
            Self::Checksum
                | Self::Size
                | Self::SourceFailed
                | Self::Listener
                | Self::IsolationFailed
                | Self::LimitMemory
                | Self::LoadCrash
                | Self::HashMismatch
                | Self::StartFailed
                | Self::ProbeFailed
                | Self::LimitDisk
                | Self::ProgramMismatch
                | Self::ArchiveRejected
        )
    }
}
