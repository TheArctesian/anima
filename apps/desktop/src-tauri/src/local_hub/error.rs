//! Why the shell could not do what was asked of the hub on this computer, as the client sees
//! it: a stable code and fixed text, never a path or an OS error.

use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::program::ProgramError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LocalHubError {
    #[error("only the main window may change the hub on this computer")]
    Caller,
    #[error("a hub cannot run on this system")]
    Unavailable,
    #[error("this build of the app does not carry the hub")]
    NotCarried,
    #[error("an install or a remove is already running")]
    Busy,
    #[error("the app could not find your home folder")]
    NoHome,
    #[error("the hub's program did not finish in time")]
    TimedOut,
    #[error("the hub's program did not finish its work; the app's log has the cause")]
    Failed,
}

impl LocalHubError {
    /// Every error, for the tests that check each one.
    #[cfg(test)]
    pub const ALL: [Self; 7] = [
        Self::Caller,
        Self::Unavailable,
        Self::NotCarried,
        Self::Busy,
        Self::NoHome,
        Self::TimedOut,
        Self::Failed,
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
