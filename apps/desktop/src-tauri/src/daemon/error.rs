//! Why the daemon was not installed, as the client sees it: a stable code and fixed text, never
//! the code, a path or an OS error (LOG-007).

use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::{placement::PlacementError, run::RunError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DaemonError {
    #[error("only the main window may install the daemon")]
    Caller,
    #[error("the daemon cannot be installed on this system")]
    Unavailable,
    #[error("this build of the app carries no daemon")]
    NotBundled,
    #[error("an install is already running")]
    Busy,
    #[error("the hub gave a code in a form the daemon does not accept")]
    Code,
    #[error("the hub address is not a plain https origin on port 443")]
    Origin,
    #[error("the app could not find your home folder")]
    NoHome,
    #[error(
        "~/.local/bin, or a folder above it, can be changed by other users (through its group, an ACL or as others): run `chmod g-w,o-w` on it, and `setfacl -b` for an ACL"
    )]
    Untrusted,
    #[error("the daemon could not be copied to ~/.local/bin")]
    Copy,
    #[error("the daemon could not enrol with the hub; make a new code and try again")]
    Enrol,
    #[error("the daemon enrolled but did not start; run `anima-daemon install` in a terminal")]
    Install,
    #[error("the daemon did not finish in time")]
    TimedOut,
    #[error("the install did not finish")]
    Failed,
}

impl DaemonError {
    /// Every error, for the tests that check each one.
    #[cfg(test)]
    pub const ALL: [Self; 13] = [
        Self::Caller,
        Self::Unavailable,
        Self::NotBundled,
        Self::Busy,
        Self::Code,
        Self::Origin,
        Self::NoHome,
        Self::Untrusted,
        Self::Copy,
        Self::Enrol,
        Self::Install,
        Self::TimedOut,
        Self::Failed,
    ];

    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Unavailable => "unavailable",
            Self::NotBundled => "not-bundled",
            Self::Busy => "busy",
            Self::Code => "code",
            Self::Origin => "origin",
            Self::NoHome => "no-home",
            Self::Untrusted => "untrusted",
            Self::Copy => "copy",
            Self::Enrol => "enrol",
            Self::Install => "install",
            Self::TimedOut => "timed-out",
            Self::Failed => "failed",
        }
    }

    pub fn of_placement(error: PlacementError) -> Self {
        match error {
            PlacementError::Untrusted => Self::Untrusted,
            PlacementError::Source => Self::NotBundled,
            PlacementError::Write => Self::Copy,
        }
    }

    /// `failed` is what the step's own failure means; the rest are the same for both steps.
    pub fn of_run(error: RunError, failed: Self) -> Self {
        match error {
            RunError::TimedOut => Self::TimedOut,
            RunError::Failed => failed,
            RunError::CouldNotStart
            | RunError::Input
            | RunError::Stopped
            | RunError::Unreadable => Self::Failed,
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for DaemonError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("DaemonError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
