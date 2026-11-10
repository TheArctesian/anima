use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::keystore::KeystoreError;

/// Why the updater key could not be imported, read or used. Carries no key text, no password and
/// no path: only the variant crosses to the client (LOG-007, BLD-045).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UpdaterKeyError {
    #[error("no key file was chosen")]
    Cancelled,
    #[error("the key file could not be read")]
    File,
    #[error("the file is not a minisign secret key")]
    Format,
    #[error("the key file is encrypted; give its password")]
    PasswordNeeded,
    #[error("the password does not open the key file")]
    WrongPassword,
    #[error("the key file has no password; set a new one to import it")]
    NewPasswordNeeded,
    #[error("a new password needs at least 12 characters")]
    PasswordTooShort,
    #[error("the stored key does not match its public key; import the key file again")]
    Mismatch,
    #[error("no updater key is stored on this computer")]
    Missing,
    #[error("the updater key is locked; unlock it with its password")]
    Locked,
    #[error("only the main window may use the updater key")]
    Caller,
    #[error("the file could not be signed or the signature does not verify")]
    Signature,
    #[error("the signature is not for this version")]
    SignedVersion,
    #[error("the OS key store failed")]
    Store(KeystoreError),
}

impl UpdaterKeyError {
    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::File => "file",
            Self::Format => "format",
            Self::PasswordNeeded => "password-needed",
            Self::WrongPassword => "password",
            Self::NewPasswordNeeded => "new-password-needed",
            Self::PasswordTooShort => "short-password",
            Self::Mismatch => "mismatch",
            Self::Missing => "missing",
            Self::Locked => "locked",
            Self::Caller => "caller",
            Self::Signature => "signature",
            Self::SignedVersion => "signed-version",
            Self::Store(KeystoreError::Unavailable) => "unavailable",
            Self::Store(KeystoreError::AccessDenied) => "denied",
            Self::Store(_) => "failed",
        }
    }
}

impl From<KeystoreError> for UpdaterKeyError {
    fn from(error: KeystoreError) -> Self {
        Self::Store(error)
    }
}

/// Serialises as `{ "code": ..., "message": ... }` with the fixed `Display` text.
impl Serialize for UpdaterKeyError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("UpdaterKeyError", 2)?;
        out.serialize_field("code", self.code())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}
