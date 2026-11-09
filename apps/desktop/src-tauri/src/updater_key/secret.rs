//! A minisign secret key, read from the key file `tauri signer generate` writes and decrypted in
//! memory (BLD-045). Pure: no file, no key store, no log.
//!
//! The key file is the minisign text (an untrusted comment line, then base64 of the key) or, as
//! Tauri writes it, base64 of that text. The key bytes are, in order: signature algorithm `Ed`,
//! KDF algorithm (`Sc` for scrypt or two zero bytes), checksum algorithm `B2`, the scrypt salt,
//! opslimit and memlimit, then the sealed part: key number, Ed25519 secret key (seed then public
//! key) and a `BLAKE2b-256` checksum. With `Sc` the sealed part is XOR-ed with an `scrypt` stream.

use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::SigningKey;
use zeroize::Zeroizing;

use super::{
    error::UpdaterKeyError,
    sealed::{checksum_holds, unseal},
};

pub(super) const SIG_ALG: &[u8] = b"Ed";
pub(super) const KDF_SCRYPT: &[u8] = b"Sc";
pub(super) const KDF_NONE: &[u8] = &[0, 0];
pub(super) const CHK_ALG: &[u8] = b"B2";
const COMMENT_PREFIX: &str = "untrusted comment: ";

pub(super) const KEYNUM_BYTES: usize = 8;
pub(super) const SEED_BYTES: usize = 32;
pub(super) const PUBLIC_BYTES: usize = 32;
pub(super) const SALT_BYTES: usize = 32;
pub(super) const CHK_BYTES: usize = 32;
/// Where the salt, the two limits and the sealed part start in the key bytes.
pub(super) const SALT_AT: usize = 6;
pub(super) const OPS_AT: usize = SALT_AT + SALT_BYTES;
pub(super) const MEM_AT: usize = OPS_AT + 8;
pub(super) const SEALED_AT: usize = MEM_AT + 8;
pub(super) const SEALED_BYTES: usize = KEYNUM_BYTES + SEED_BYTES + PUBLIC_BYTES + CHK_BYTES;
pub(super) const KEY_BYTES: usize = SEALED_AT + SEALED_BYTES;

pub(super) const PASSWORD_MAX_BYTES: usize = 1024;

/// The updater secret key. No `Debug`, no `Clone`, no `Display`: the seed is only ever read to sign
/// or to store it, and is wiped when the value drops (LOG-007).
pub(crate) struct SecretKey {
    keynum: [u8; KEYNUM_BYTES],
    seed: Zeroizing<[u8; SEED_BYTES]>,
    public: [u8; PUBLIC_BYTES],
}

/// A minisign public key: its key number and Ed25519 key. Public; safe to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PublicKey {
    keynum: [u8; KEYNUM_BYTES],
    key: [u8; PUBLIC_BYTES],
}

impl PublicKey {
    /// The key line of a minisign public key file (`RWQ...`), as `minisign -p` and
    /// `minisign-verify` read it.
    pub(crate) fn line(&self) -> String {
        let mut bytes = Vec::with_capacity(2 + KEYNUM_BYTES + PUBLIC_BYTES);
        bytes.extend_from_slice(SIG_ALG);
        bytes.extend_from_slice(&self.keynum);
        bytes.extend_from_slice(&self.key);
        STANDARD.encode(bytes)
    }

    /// Reads what `line` wrote; anything else is `Format`.
    pub(crate) fn from_line(text: &str) -> Result<Self, UpdaterKeyError> {
        let bytes = STANDARD
            .decode(text.trim())
            .map_err(|_| UpdaterKeyError::Format)?;
        if bytes.len() != 2 + KEYNUM_BYTES + PUBLIC_BYTES || &bytes[..2] != SIG_ALG {
            return Err(UpdaterKeyError::Format);
        }
        let mut keynum = [0; KEYNUM_BYTES];
        keynum.copy_from_slice(&bytes[2..2 + KEYNUM_BYTES]);
        let mut key = [0; PUBLIC_BYTES];
        key.copy_from_slice(&bytes[2 + KEYNUM_BYTES..]);
        Ok(Self { keynum, key })
    }

    pub(crate) fn keynum(&self) -> [u8; KEYNUM_BYTES] {
        self.keynum
    }
}

impl SecretKey {
    /// Reads a key file. `password` opens an encrypted file; a file encrypted with an empty
    /// password (what `tauri signer generate` makes without one) opens with `None`.
    pub(crate) fn from_key_file(
        file: &[u8],
        password: Option<&str>,
    ) -> Result<Self, UpdaterKeyError> {
        let mut bytes = key_bytes(file)?;
        let kdf = &bytes[2..4];
        if kdf == KDF_SCRYPT {
            let given = password.unwrap_or("");
            if given.len() > PASSWORD_MAX_BYTES {
                return Err(UpdaterKeyError::WrongPassword);
            }
            unseal(&mut bytes, given.as_bytes())?;
            if !checksum_holds(&bytes) {
                return Err(if password.is_some() {
                    UpdaterKeyError::WrongPassword
                } else {
                    UpdaterKeyError::PasswordNeeded
                });
            }
        } else if kdf != KDF_NONE || !checksum_holds(&bytes) {
            return Err(UpdaterKeyError::Format);
        }
        Self::from_sealed(&bytes[SEALED_AT..])
    }

    /// Key number, seed and public key; the public key must be the seed's.
    pub(super) fn from_sealed(sealed: &[u8]) -> Result<Self, UpdaterKeyError> {
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
            return Err(UpdaterKeyError::Format);
        }
        Ok(key)
    }

    pub(crate) fn public_key(&self) -> PublicKey {
        PublicKey {
            keynum: self.keynum,
            key: self.public,
        }
    }

    /// Key number, seed and public key, in the order minisign seals them. Wiped when dropped.
    pub(super) fn sealed_body(&self) -> Zeroizing<Vec<u8>> {
        let mut body = Zeroizing::new(Vec::with_capacity(KEYNUM_BYTES + SEED_BYTES + PUBLIC_BYTES));
        body.extend_from_slice(&self.keynum);
        body.extend_from_slice(self.seed.as_slice());
        body.extend_from_slice(&self.public);
        body
    }

    /// The Ed25519 signing key; it wipes itself when dropped.
    pub(crate) fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.seed)
    }
}

/// The 158 key bytes of a key file in either form. Lengths are checked, never trusted.
pub(super) fn key_bytes(file: &[u8]) -> Result<Zeroizing<Vec<u8>>, UpdaterKeyError> {
    let text = std::str::from_utf8(file).map_err(|_| UpdaterKeyError::Format)?;
    let text = text.trim();
    let minisign = if text.starts_with(COMMENT_PREFIX) {
        Zeroizing::new(text.to_owned())
    } else {
        let decoded = Zeroizing::new(STANDARD.decode(text).map_err(|_| UpdaterKeyError::Format)?);
        Zeroizing::new(
            std::str::from_utf8(&decoded)
                .map_err(|_| UpdaterKeyError::Format)?
                .to_owned(),
        )
    };
    let mut lines = minisign.lines();
    let comment = lines.next().unwrap_or_default();
    let encoded = lines.next().ok_or(UpdaterKeyError::Format)?;
    if !comment.starts_with(COMMENT_PREFIX) {
        return Err(UpdaterKeyError::Format);
    }
    let bytes = Zeroizing::new(
        STANDARD
            .decode(encoded.trim())
            .map_err(|_| UpdaterKeyError::Format)?,
    );
    let shaped =
        bytes.len() == KEY_BYTES && &bytes[..2] == SIG_ALG && &bytes[4..SALT_AT] == CHK_ALG;
    if !shaped {
        return Err(UpdaterKeyError::Format);
    }
    Ok(bytes)
}
