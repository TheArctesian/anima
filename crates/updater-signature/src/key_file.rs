//! A minisign key pair: the secret key read from the key file `tauri signer generate` writes and
//! decrypted in memory, and the public key (BLD-045, R-117). Pure: no file, no store, no log.
//!
//! The key file is the minisign text (an untrusted comment line, then base64 of the key) or, as
//! Tauri writes it, base64 of that text. The key bytes are, in order: signature algorithm `Ed`,
//! KDF algorithm (`Sc` for scrypt or two zero bytes), checksum algorithm `B2`, the scrypt salt,
//! opslimit and memlimit, then the sealed part: key number, Ed25519 secret key (seed then public
//! key) and a `BLAKE2b-256` checksum. With `Sc` the sealed part is XOR-ed with an `scrypt` stream.

use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::SigningKey;
use zeroize::Zeroizing;

use crate::{
    error::SignatureError,
    sealed::{checksum_holds, unseal},
};

pub(crate) const SIG_ALG: &[u8] = b"Ed";
pub(crate) const KDF_SCRYPT: &[u8] = b"Sc";
const KDF_NONE: &[u8] = &[0, 0];
pub(crate) const CHK_ALG: &[u8] = b"B2";
const COMMENT_PREFIX: &str = "untrusted comment: ";

pub(crate) const KEYNUM_BYTES: usize = 8;
pub(crate) const SEED_BYTES: usize = 32;
pub(crate) const PUBLIC_BYTES: usize = 32;
pub(crate) const SALT_BYTES: usize = 32;
pub(crate) const CHK_BYTES: usize = 32;
/// Where the salt, the two limits and the sealed part start in the key bytes.
pub(crate) const SALT_AT: usize = 6;
pub(crate) const OPS_AT: usize = SALT_AT + SALT_BYTES;
pub(crate) const MEM_AT: usize = OPS_AT + 8;
pub(crate) const SEALED_AT: usize = MEM_AT + 8;
pub(crate) const SEALED_BYTES: usize = KEYNUM_BYTES + SEED_BYTES + PUBLIC_BYTES + CHK_BYTES;
pub(crate) const KEY_BYTES: usize = SEALED_AT + SEALED_BYTES;

pub(crate) const PASSWORD_MAX_BYTES: usize = 1024;

/// The updater secret key. No `Debug`, no `Clone`, no `Display`: the seed is only ever read to
/// sign, and is wiped when the value drops (LOG-007).
pub struct SecretKey {
    keynum: [u8; KEYNUM_BYTES],
    seed: Zeroizing<[u8; SEED_BYTES]>,
    public: [u8; PUBLIC_BYTES],
}

/// A minisign public key: its key number and Ed25519 key. Public; safe to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey {
    keynum: [u8; KEYNUM_BYTES],
    key: [u8; PUBLIC_BYTES],
}

impl PublicKey {
    /// The key line of a minisign public key file (`RWQ...`), as `minisign -p` and
    /// `minisign-verify` read it.
    #[must_use]
    pub(crate) fn line(&self) -> String {
        let mut bytes = Vec::with_capacity(2 + KEYNUM_BYTES + PUBLIC_BYTES);
        bytes.extend_from_slice(SIG_ALG);
        bytes.extend_from_slice(&self.keynum);
        bytes.extend_from_slice(&self.key);
        STANDARD.encode(bytes)
    }

    /// Reads what `line` wrote; anything else is `Format`.
    ///
    /// # Errors
    ///
    /// `Format` for anything but one key line.
    pub(crate) fn from_line(text: &str) -> Result<Self, SignatureError> {
        let bytes = STANDARD
            .decode(text.trim())
            .map_err(|_| SignatureError::Format)?;
        if bytes.len() != 2 + KEYNUM_BYTES + PUBLIC_BYTES || &bytes[..2] != SIG_ALG {
            return Err(SignatureError::Format);
        }
        let mut keynum = [0; KEYNUM_BYTES];
        keynum.copy_from_slice(&bytes[2..2 + KEYNUM_BYTES]);
        let mut key = [0; PUBLIC_BYTES];
        key.copy_from_slice(&bytes[2 + KEYNUM_BYTES..]);
        Ok(Self { keynum, key })
    }

    /// Reads a public key file, as minisign writes it (an untrusted comment line, then the key
    /// line) or, as `plugins.updater.pubkey` and `ANIMA_UPDATER_PUBLIC_KEY` hold it, base64 of
    /// that text. The last line that is not blank is the key line.
    ///
    /// # Errors
    ///
    /// `Format` for anything else.
    pub fn from_key_file(text: &str) -> Result<Self, SignatureError> {
        let text = text.trim();
        let decoded;
        let file = if text.starts_with(COMMENT_PREFIX) {
            text
        } else {
            let bytes = STANDARD.decode(text).map_err(|_| SignatureError::Format)?;
            decoded = String::from_utf8(bytes).map_err(|_| SignatureError::Format)?;
            decoded.as_str()
        };
        let mut lines = file.lines().map(str::trim).filter(|line| !line.is_empty());
        let comment = lines.next().ok_or(SignatureError::Format)?;
        let line = lines.next().ok_or(SignatureError::Format)?;
        if !comment.starts_with(COMMENT_PREFIX) || lines.next().is_some() {
            return Err(SignatureError::Format);
        }
        Self::from_line(line)
    }

    pub(crate) fn keynum(&self) -> [u8; KEYNUM_BYTES] {
        self.keynum
    }
}

impl SecretKey {
    /// Reads a key file. `password` opens an encrypted file; a file encrypted with an empty
    /// password (what `tauri signer generate` makes without one) opens with `None` or `""`.
    ///
    /// # Errors
    ///
    /// `Format` for a file that is not a minisign secret key, or asks for a scrypt cost above
    /// N = 2^20 or p = 1; `WrongPassword` when `password` does not open it; `PasswordNeeded`
    /// when it is encrypted and no password was given.
    pub fn from_key_file(file: &[u8], password: Option<&str>) -> Result<Self, SignatureError> {
        let mut bytes = key_bytes(file)?;
        let kdf = &bytes[2..4];
        if kdf == KDF_SCRYPT {
            let given = password.unwrap_or("");
            if given.len() > PASSWORD_MAX_BYTES {
                return Err(SignatureError::WrongPassword);
            }
            unseal(&mut bytes, given.as_bytes())?;
            if !checksum_holds(&bytes) {
                return Err(if password.is_some() {
                    SignatureError::WrongPassword
                } else {
                    SignatureError::PasswordNeeded
                });
            }
        } else if kdf != KDF_NONE || !checksum_holds(&bytes) {
            return Err(SignatureError::Format);
        }
        Self::from_sealed(&bytes[SEALED_AT..])
    }

    /// Key number, seed and public key; the public key must be the seed's.
    fn from_sealed(sealed: &[u8]) -> Result<Self, SignatureError> {
        let mut keynum = [0; KEYNUM_BYTES];
        keynum.copy_from_slice(&sealed[..KEYNUM_BYTES]);
        let mut seed = Zeroizing::new([0; SEED_BYTES]);
        seed.copy_from_slice(&sealed[KEYNUM_BYTES..KEYNUM_BYTES + SEED_BYTES]);
        let mut public = [0; PUBLIC_BYTES];
        public.copy_from_slice(
            &sealed[KEYNUM_BYTES + SEED_BYTES..KEYNUM_BYTES + SEED_BYTES + PUBLIC_BYTES],
        );
        let key = Self {
            keynum,
            seed,
            public,
        };
        if key.signing_key().verifying_key().to_bytes() != key.public {
            return Err(SignatureError::Format);
        }
        Ok(key)
    }

    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            keynum: self.keynum,
            key: self.public,
        }
    }

    /// The Ed25519 signing key; it wipes itself when dropped.
    pub(crate) fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.seed)
    }
}

/// The 158 key bytes of a key file in either form. Lengths are checked, never trusted.
fn key_bytes(file: &[u8]) -> Result<Zeroizing<Vec<u8>>, SignatureError> {
    let text = std::str::from_utf8(file).map_err(|_| SignatureError::Format)?;
    let text = text.trim();
    let minisign = if text.starts_with(COMMENT_PREFIX) {
        Zeroizing::new(text.to_owned())
    } else {
        let decoded = Zeroizing::new(STANDARD.decode(text).map_err(|_| SignatureError::Format)?);
        Zeroizing::new(
            std::str::from_utf8(&decoded)
                .map_err(|_| SignatureError::Format)?
                .to_owned(),
        )
    };
    let mut lines = minisign.lines();
    let comment = lines.next().unwrap_or_default();
    let encoded = lines.next().ok_or(SignatureError::Format)?;
    if !comment.starts_with(COMMENT_PREFIX) {
        return Err(SignatureError::Format);
    }
    let bytes = Zeroizing::new(
        STANDARD
            .decode(encoded.trim())
            .map_err(|_| SignatureError::Format)?,
    );
    let shaped =
        bytes.len() == KEY_BYTES && &bytes[..2] == SIG_ALG && &bytes[4..SALT_AT] == CHK_ALG;
    if !shaped {
        return Err(SignatureError::Format);
    }
    Ok(bytes)
}
