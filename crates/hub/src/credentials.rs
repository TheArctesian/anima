//! Device credentials: secrets sealed with the hub key before they reach storage (SEC-003).
//!
//! [`Vault`] holds the key from `ANIMA_KEY_FILE` (SEC-004) and seals each [`Secret`] with
//! XChaCha20-Poly1305 into a [`Sealed`] value. Plaintext never leaves this module except
//! through [`Secret::expose`]; secret types implement no format or serialise traits, and
//! this module writes no logs (LOG-007).

mod secret;
mod vault;

#[cfg(test)]
mod tests;

pub use secret::Secret;
pub use vault::{Sealed, Vault};

/// Why a credential operation failed. Messages name no path, key, nonce or plaintext.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The key file could not be read.
    #[error("the credential key file cannot be read ({0})")]
    KeyFile(std::io::ErrorKind),
    /// The key file does not hold a base64-encoded 32-byte key.
    #[error("the credential key file does not hold a base64-encoded 32-byte key")]
    KeyFormat,
    /// The sealed value names a key other than the vault's.
    #[error("the secret was sealed with a different key")]
    WrongKey,
    /// Authentication failed: the key, nonce, ciphertext or context does not match.
    #[error("the secret cannot be decrypted")]
    Decrypt,
    /// The sealed value is not in a known layout.
    #[error("the sealed secret is malformed")]
    Malformed,
}
