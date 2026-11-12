/// Why a key file could not be opened or a file could not be signed or checked. Carries no key
/// text, no password, no path and no signature: only the variant and its fixed text (LOG-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    #[error("the file is not a minisign key")]
    Format,
    #[error("the key file is encrypted and needs its password")]
    PasswordNeeded,
    #[error("the password does not open the key file")]
    WrongPassword,
    #[error("the file could not be signed or the signature does not verify")]
    Signature,
    #[error("the signature is not for this version")]
    SignedVersion,
    #[error("the signed file could not be read")]
    Read,
}
