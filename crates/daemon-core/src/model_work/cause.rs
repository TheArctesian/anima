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
    /// Linux: the tree of a server cannot be contained; the detail says which step failed.
    /// There is no start with less containment (R-NEW-containment, R-NEW-cgroupns).
    ContainmentFailed(Containment),
}

/// The step at which the containment of a server failed: the detail of `containment_failed`.
/// The launcher steps each have their own exit code, so a refusal of the user namespace reads
/// as such.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Containment {
    /// No cgroup v2, or the cgroup of the daemon is not delegated to its user.
    NoDelegation,
    /// The cgroup v2 mount has no `nsdelegate`, so a namespace would not stop a move out.
    NoNsdelegate,
    /// The launcher could not write itself into the cgroup of the server.
    CgroupJoinFailed,
    /// The launcher could not set the parent-death signal.
    BackstopFailed,
    /// The daemon ended before the launcher set the parent-death signal.
    ParentGone,
    /// The system refused the user and cgroup namespaces (`AppArmor` or a sysctl).
    UsernsDenied,
    /// The launcher could not run the runtime, or its arguments were wrong.
    ExecFailed,
}

impl Containment {
    const ALL: [Self; 5] = [
        Self::CgroupJoinFailed,
        Self::BackstopFailed,
        Self::ParentGone,
        Self::UsernsDenied,
        Self::ExecFailed,
    ];

    /// The detail as the report spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoDelegation => "no_delegation",
            Self::NoNsdelegate => "no_nsdelegate",
            Self::CgroupJoinFailed => "cgroup_join_failed",
            Self::BackstopFailed => "backstop_failed",
            Self::ParentGone => "parent_gone",
            Self::UsernsDenied => "userns_denied",
            Self::ExecFailed => "exec_failed",
        }
    }

    /// The exit code of the launcher for a failure at this step; `None` for a step of the
    /// daemon, before the launcher runs.
    #[must_use]
    pub const fn exit_code(self) -> Option<i32> {
        match self {
            Self::NoDelegation | Self::NoNsdelegate => None,
            Self::CgroupJoinFailed => Some(120),
            Self::BackstopFailed => Some(121),
            Self::ParentGone => Some(122),
            Self::UsernsDenied => Some(123),
            Self::ExecFailed => Some(124),
        }
    }

    /// The step of a launcher exit code.
    #[must_use]
    pub fn of_exit_code(code: i32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|step| step.exit_code() == Some(code))
    }
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
            Self::ContainmentFailed(_) => "containment_failed",
        }
    }

    /// The detail of the cause, when it has one.
    #[must_use]
    pub const fn detail(self) -> Option<&'static str> {
        match self {
            Self::ContainmentFailed(step) => Some(step.as_str()),
            _ => None,
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
                | Self::ContainmentFailed(_)
        )
    }
}
