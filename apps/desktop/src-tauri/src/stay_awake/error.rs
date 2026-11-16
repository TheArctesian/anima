//! Why a stay-awake command failed, as the client sees it: a stable code and fixed text, never
//! a path or an OS error (LOG-007).

use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::{control::ControlError, lid_set_up::SetUpError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StayAwakeError {
    #[error("only the main window may change the stay-awake control")]
    Caller,
    #[error("the choice could not be saved")]
    Unsaved,
    #[error("this system has no set-up step")]
    Unsupported,
    #[error("the administrator step was cancelled or did not finish")]
    Refused,
    #[error("the stay-awake control could not be changed")]
    Failed,
}

impl StayAwakeError {
    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Unsaved => "unsaved",
            Self::Unsupported => "unsupported",
            Self::Refused => "refused",
            Self::Failed => "failed",
        }
    }
}

impl From<ControlError> for StayAwakeError {
    fn from(error: ControlError) -> Self {
        match error {
            ControlError::Unsaved => Self::Unsaved,
            ControlError::SetUp(SetUpError::Unsupported) => Self::Unsupported,
            ControlError::SetUp(SetUpError::Refused) => Self::Refused,
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for StayAwakeError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("StayAwakeError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
