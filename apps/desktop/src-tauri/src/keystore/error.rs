use serde::{Serialize, Serializer, ser::SerializeStruct};

/// Why a key-store call failed. Carries no token and no backend text: the backend's message
/// can echo entry attributes (LOG-007), so only the variant crosses to the client.
///
/// What each code means, by platform:
/// - `unavailable`: the store cannot be used. Linux: no Secret Service on the session bus.
///   macOS: any Keychain failure that is not one of the cases below.
/// - `denied`: the store exists and refused. Linux: the collection is locked or unlock was
///   dismissed. macOS: the operator cancelled or failed the Keychain prompt (-128, -25293), or
///   the keychain is missing, read-only or invalid.
/// - `failed`: anything else, including data the store returned in an unexpected form.
/// - `empty` and `origin`: input refused before the store was touched.
/// - `caller`: a web view other than the main one asked for the builder token (C-5).
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
    #[error("only the main window may use the builder token")]
    Caller,
}

impl KeystoreError {
    /// Stable code the client switches on.
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::AccessDenied => "denied",
            Self::EmptyToken => "empty",
            Self::BadOrigin => "origin",
            Self::Failed => "failed",
            Self::Caller => "caller",
        }
    }
}

/// keyring 3.6 reports a cancelled or failed Keychain prompt as a generic platform failure.
#[cfg(target_os = "macos")]
fn keychain_prompt_refused(error: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    const USER_CANCELLED: i32 = -128; // userCanceledErr
    const AUTH_FAILED: i32 = -25293; // errSecAuthFailed
    error
        .downcast_ref::<security_framework::base::Error>()
        .is_some_and(|e| matches!(e.code(), USER_CANCELLED | AUTH_FAILED))
}

#[cfg(not(target_os = "macos"))]
fn keychain_prompt_refused(_: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    false
}

impl From<keyring::Error> for KeystoreError {
    /// Drops the backend error on purpose; nothing from it is kept or logged.
    fn from(error: keyring::Error) -> Self {
        match error {
            keyring::Error::PlatformFailure(ref cause)
                if keychain_prompt_refused(cause.as_ref()) =>
            {
                Self::AccessDenied
            }
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
