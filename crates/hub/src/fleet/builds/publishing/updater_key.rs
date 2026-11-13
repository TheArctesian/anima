//! The updater key the hub signs releases with (R-117, BLD-045, SEC-004): the minisign secret
//! key file `tauri signer generate` writes, sealed with scrypt, opened once at start from its
//! Docker secret with the password in a second secret, and kept only if its public key is
//! `ANIMA_UPDATER_PUBLIC_KEY`. The type has no `Debug`, `Display` or `Clone`; the seed is wiped
//! when it drops, and an error names a file or a check, never a value (LOG-007).

use std::{io::Read as _, path::Path};

use updater_signature::{PublicKey, SecretKey, SignatureCheck, SignatureError, Signed};
use zeroize::Zeroizing;

use crate::config::Releases;

/// The largest key file the hub reads; a `tauri signer generate` file is about 470 bytes.
const MAX_KEY_FILE_BYTES: usize = 4096;

/// The largest password file the hub reads; `updater-signature` takes at most 1024 bytes.
const MAX_PASSWORD_FILE_BYTES: usize = 1026;

/// Why the hub does not sign: publishing is then off, and the hub still starts (R-117).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UpdaterKeyError {
    #[error("ANIMA_UPDATER_PUBLIC_KEY is unset")]
    NoPublicKey,
    #[error("ANIMA_UPDATER_PUBLIC_KEY is not a minisign public key")]
    PublicKey,
    #[error("the updater key file cannot be read ({0})")]
    KeyFile(std::io::ErrorKind),
    #[error("the updater key password file cannot be read ({0})")]
    PasswordFile(std::io::ErrorKind),
    #[error("the updater key file does not open: {0}")]
    Open(SignatureError),
    #[error("the updater key is not the key of ANIMA_UPDATER_PUBLIC_KEY")]
    Mismatch,
}

/// The opened updater key and its public key, which equals `ANIMA_UPDATER_PUBLIC_KEY`.
pub struct UpdaterKey {
    secret: SecretKey,
    public: PublicKey,
}

/// At most `max` bytes of `path`, in a buffer that never grows and is wiped when dropped.
fn read_secret(path: &Path, max: usize) -> std::io::Result<Zeroizing<Vec<u8>>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(max + 1));
    std::fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(bytes)
}

impl UpdaterKey {
    /// Reads the key and password files named in `releases` and opens the key (I/O at the edge).
    ///
    /// # Errors
    ///
    /// As [`UpdaterKey::from_parts`]; `KeyFile` or `PasswordFile` when a file cannot be read or
    /// is too large.
    pub fn open(releases: &Releases) -> Result<Self, UpdaterKeyError> {
        let public = releases
            .updater_public_key
            .as_deref()
            .ok_or(UpdaterKeyError::NoPublicKey)?;
        let key_file = read_secret(&releases.updater_key_file, MAX_KEY_FILE_BYTES)
            .map_err(|e| UpdaterKeyError::KeyFile(e.kind()))?;
        let password = read_secret(&releases.updater_key_password_file, MAX_PASSWORD_FILE_BYTES)
            .map_err(|e| UpdaterKeyError::PasswordFile(e.kind()))?;
        Self::from_parts(&key_file, &password, public)
    }

    /// Opens `key_file` with the bytes of its `password` file and checks that its public key is
    /// `public`, the `ANIMA_UPDATER_PUBLIC_KEY` text. The password file may be empty, for a key
    /// sealed with no password, and one line break at its end is not part of the password.
    ///
    /// # Errors
    ///
    /// `PublicKey` when `public` is not a key, `PasswordFile` when the password is not UTF-8,
    /// `Open` when the key file does not open with it, `Mismatch` when it opens to another key.
    pub fn from_parts(
        key_file: &[u8],
        password: &[u8],
        public: &str,
    ) -> Result<Self, UpdaterKeyError> {
        let wanted = PublicKey::from_key_file(public).map_err(|_| UpdaterKeyError::PublicKey)?;
        let password = std::str::from_utf8(password)
            .map_err(|_| UpdaterKeyError::PasswordFile(std::io::ErrorKind::InvalidData))?;
        let password = password
            .strip_suffix('\n')
            .map_or(password, |p| p.strip_suffix('\r').unwrap_or(p));
        let password = (!password.is_empty()).then_some(password);
        let secret = SecretKey::from_key_file(key_file, password).map_err(UpdaterKeyError::Open)?;
        if secret.public_key() != wanted {
            return Err(UpdaterKeyError::Mismatch);
        }
        Ok(Self {
            secret,
            public: wanted,
        })
    }

    /// The `.sig` text of the file whose BLAKE2b-512 hash is `digest`; the same digest and
    /// `signed` always give the same text (BLD-046).
    pub(super) fn sign(
        &self,
        digest: &[u8; 64],
        signed: &Signed<'_>,
    ) -> Result<String, SignatureError> {
        self.secret.sign(digest, signed)
    }

    /// A check of `signature` with the public key, by the updater's own verifier (BLD-046).
    pub(super) fn check(&self, signature: &str) -> Result<SignatureCheck, SignatureError> {
        SignatureCheck::new(&self.public, signature)
    }
}
