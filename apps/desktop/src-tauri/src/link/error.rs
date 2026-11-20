//! Why the shell could not do what was asked of this device's link, as the client sees it: a
//! stable code and fixed text, never an invite, a path or an OS error.

use serde::{Serialize, Serializer, ser::SerializeStruct};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LinkError {
    #[error("only the main window may change this device's link")]
    Caller,
    #[error("this build of the app does not carry the device link")]
    NotCarried,
    #[error("a join or a leave is already running")]
    Busy,
    #[error("the app could not find your home folder")]
    NoHome,
    #[error("this is not an invite: it must start with anima-invite.1.")]
    BadInvite,
    #[error("the hub did not answer: the invite is used, expired or wrong, or the hub is off")]
    NoAnswer,
    #[error("the hub refused this device; ask for a new invite")]
    Refused,
    #[error("the link's folder could not be made or changed")]
    Folder,
    #[error("the device link did not finish its work")]
    Failed,
}

impl LinkError {
    /// Every error, for the tests that check each one.
    #[cfg(all(test, unix))]
    pub const ALL: [Self; 9] = [
        Self::Caller,
        Self::NotCarried,
        Self::Busy,
        Self::NoHome,
        Self::BadInvite,
        Self::NoAnswer,
        Self::Refused,
        Self::Folder,
        Self::Failed,
    ];

    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::NotCarried => "not-carried",
            Self::Busy => "busy",
            Self::NoHome => "no-home",
            Self::BadInvite => "bad-invite",
            Self::NoAnswer => "no-answer",
            Self::Refused => "refused",
            Self::Folder => "folder",
            Self::Failed => "failed",
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for LinkError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("LinkError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
