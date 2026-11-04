//! SSH public key blobs and their OpenSSH SHA-256 fingerprints (SEC-006, SEC-007).
//!
//! Pure. A key blob comes from a remote host or from the database, so it is parsed with every
//! length checked; nothing here panics on any input.

use std::{fmt, str::FromStr};

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use sha2::{Digest as _, Sha256};

/// Largest key blob accepted. A 16384-bit RSA key is about 2 KiB.
const MAX_BLOB: usize = 8 * 1024;
/// Longest algorithm name accepted (OpenSSH names are well under this).
const MAX_ALGORITHM: usize = 64;
const FINGERPRINT_PREFIX: &str = "SHA256:";

/// Why bytes or text are not an SSH public key. Never holds the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BadKey {
    #[error("the key is longer than the hub accepts")]
    TooLong,
    #[error("the key is not valid base64")]
    NotBase64,
    #[error("the key ends inside its algorithm name")]
    Truncated,
    #[error("the key's algorithm name is not valid")]
    BadAlgorithm,
    #[error("the key has no key data after its algorithm name")]
    NoKeyData,
}

/// An SSH public key blob (RFC 4253 section 6.6): a length-prefixed algorithm name, then the
/// key data. The algorithm name is checked to hold only `[A-Za-z0-9@._+-]`, so it can go into
/// a `known_hosts` line as it is.
#[derive(Clone, PartialEq, Eq)]
pub struct PublicKey {
    blob: Vec<u8>,
    /// Length of the algorithm name, which starts at byte 4.
    algorithm_len: usize,
}

impl PublicKey {
    /// Parses a key blob.
    ///
    /// # Errors
    /// [`BadKey`] when the blob is too long, ends early, or has a bad algorithm name.
    pub fn from_blob(blob: Vec<u8>) -> Result<Self, BadKey> {
        if blob.len() > MAX_BLOB {
            return Err(BadKey::TooLong);
        }
        let (length, rest) = blob.split_first_chunk::<4>().ok_or(BadKey::Truncated)?;
        let algorithm_len =
            usize::try_from(u32::from_be_bytes(*length)).map_err(|_| BadKey::Truncated)?;
        let name = rest.get(..algorithm_len).ok_or(BadKey::Truncated)?;
        if !is_algorithm_name(name) {
            return Err(BadKey::BadAlgorithm);
        }
        if rest.len() == algorithm_len {
            return Err(BadKey::NoKeyData);
        }
        Ok(Self {
            blob,
            algorithm_len,
        })
    }

    /// Parses the standard base64 text of a key blob, as in a `known_hosts` line.
    ///
    /// # Errors
    /// [`BadKey`] when the text is not canonical base64 or not a key blob.
    pub fn from_base64(text: &str) -> Result<Self, BadKey> {
        if text.len() > MAX_BLOB.div_ceil(3) * 4 {
            return Err(BadKey::TooLong);
        }
        let blob = STANDARD.decode(text).map_err(|_| BadKey::NotBase64)?;
        Self::from_blob(blob)
    }

    /// The algorithm name inside the blob, for example `ssh-ed25519`.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        // from_blob checked these bytes are ASCII.
        std::str::from_utf8(&self.blob[4..4 + self.algorithm_len]).unwrap_or_default()
    }

    #[must_use]
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// The blob in standard base64 with padding, as OpenSSH writes it.
    #[must_use]
    pub fn to_base64(&self) -> String {
        STANDARD.encode(&self.blob)
    }

    #[must_use]
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of_key(&self.blob)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PublicKey")
            .field("algorithm", &self.algorithm())
            .field("fingerprint", &self.fingerprint().as_str())
            .finish()
    }
}

fn is_algorithm_name(name: &[u8]) -> bool {
    (1..=MAX_ALGORITHM).contains(&name.len())
        && name
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b"@._+-".contains(&b))
}

/// OpenSSH's SHA-256 fingerprint text of a key blob: `SHA256:` and the digest in base64
/// without padding (what `ssh-keygen -l` prints).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fingerprint(String);

/// The text is not an OpenSSH SHA-256 fingerprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not an OpenSSH SHA256 fingerprint")]
pub struct BadFingerprint;

impl Fingerprint {
    /// The fingerprint of a key blob.
    #[must_use]
    pub fn of_key(blob: &[u8]) -> Self {
        Self(format!(
            "{FINGERPRINT_PREFIX}{}",
            STANDARD_NO_PAD.encode(Sha256::digest(blob))
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Fingerprint {
    type Err = BadFingerprint;

    /// Accepts exactly the text [`Fingerprint::of_key`] makes: the prefix and 43 characters of
    /// canonical base64 that decode to 32 bytes.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digest = text
            .strip_prefix(FINGERPRINT_PREFIX)
            .filter(|digest| digest.len() == 43)
            .ok_or(BadFingerprint)?;
        match STANDARD_NO_PAD.decode(digest) {
            Ok(bytes) if bytes.len() == 32 => Ok(Self(text.to_owned())),
            _ => Err(BadFingerprint),
        }
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
