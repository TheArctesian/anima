//! Why the shell could not do what was asked of the hub on this computer, as the client sees
//! it: a stable code and fixed text, never a path or an OS error.

use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::carried::run::ProgramError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LocalHubError {
    #[error("only the main window may change the hub on this computer")]
    Caller,
    #[error("a hub cannot run on this system")]
    Unavailable,
    #[error("this build of the app does not carry the hub")]
    NotCarried,
    #[error("another change of the hub on this computer is running")]
    Busy,
    #[error("the app could not find your home folder")]
    NoHome,
    #[error("the hub's program did not finish in time")]
    TimedOut,
    #[error("the hub's program did not finish its work; the app's log has the cause")]
    Failed,
    #[error("a value is empty, too long or not of its form")]
    Argument,
    #[error("the account was not replaced: the confirmation was cancelled")]
    Cancelled,
    #[error("the hub refused the account; check the code and try again")]
    Refused,
    #[error("the hub on this computer has no key file")]
    NoKey,
    #[error("the key could not be saved at that place")]
    Destination,
}

impl LocalHubError {
    /// Every error, for the tests that check each one.
    #[cfg(all(test, unix))]
    pub const ALL: [Self; 12] = [
        Self::Caller,
        Self::Unavailable,
        Self::NotCarried,
        Self::Busy,
        Self::NoHome,
        Self::TimedOut,
        Self::Failed,
        Self::Argument,
        Self::Cancelled,
        Self::Refused,
        Self::NoKey,
        Self::Destination,
    ];

    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Unavailable => "unavailable",
            Self::NotCarried => "not-carried",
            Self::Busy => "busy",
            Self::NoHome => "no-home",
            Self::TimedOut => "timed-out",
            Self::Failed => "failed",
            Self::Argument => "argument",
            Self::Cancelled => "cancelled",
            Self::Refused => "refused",
            Self::NoKey => "no-key",
            Self::Destination => "destination",
        }
    }
}

impl From<&ProgramError> for LocalHubError {
    fn from(error: &ProgramError) -> Self {
        match error {
            ProgramError::TimedOut => Self::TimedOut,
            ProgramError::CouldNotStart | ProgramError::Failed(_) | ProgramError::Unreadable => {
                Self::Failed
            }
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for LocalHubError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("LocalHubError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
