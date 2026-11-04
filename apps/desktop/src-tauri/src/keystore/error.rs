use serde::{Serialize, Serializer, ser::SerializeStruct};

/// Why a key-store call failed. Carries no token and no backend text: the backend's message
/// can echo entry attributes (LOG-007), so only the variant crosses to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum KeystoreError {
    #[error("the OS key store is not available")]
    Unavailable,
    #[error("the OS key store refused access")]
    AccessDenied,
    #[error("the token is empty")]
    EmptyToken,
    #[error("the hub origin is not an https origin")]
    BadOrigin,
    #[error("the OS key store failed")]
    Failed,
}

impl KeystoreError {
    /// Stable code the client switches on.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::AccessDenied => "denied",
            Self::EmptyToken => "empty",
            Self::BadOrigin => "origin",
            Self::Failed => "failed",
        }
    }
}

impl From<keyring::Error> for KeystoreError {
    /// Drops the backend error on purpose; nothing from it is kept or logged.
    fn from(error: keyring::Error) -> Self {
        match error {
            keyring::Error::PlatformFailure(_) => Self::Unavailable,
            keyring::Error::NoStorageAccess(_) => Self::AccessDenied,
            _ => Self::Failed,
        }
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for KeystoreError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("KeystoreError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
