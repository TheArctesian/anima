//! The hub key and the sealed form of a secret (SEC-003, SEC-004).

use std::{fmt, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce, aead::AeadInOut};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::{Error, Secret};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const KEY_ID_BYTES: usize = 8;
const KEY_ID_DOMAIN: &[u8] = b"anima/credentials/key-id/v1";
const SEALED_TAG: &[u8] = b"anima/credentials/sealed/v1";
const LAYOUT_V1: u8 = 0x01;

/// The hub's credential key: seals and opens secrets with XChaCha20-Poly1305.
///
/// Each clone holds its own copy of the key, zeroized on drop. `Debug` prints only the key id.
#[derive(Clone)]
pub struct Vault {
    key: Zeroizing<[u8; KEY_LEN]>,
    key_id: String,
}

impl Vault {
    /// Reads the key from `ANIMA_KEY_FILE`: 32 bytes as standard base64, surrounding
    /// whitespace ignored (SEC-004).
    ///
    /// # Errors
    ///
    /// [`Error::KeyFile`] when the file cannot be read, [`Error::KeyFormat`] when it does
    /// not hold exactly one base64-encoded 32-byte key.
    pub fn from_key_file(path: &Path) -> Result<Self, Error> {
        let text =
            Zeroizing::new(std::fs::read(path).map_err(|error| Error::KeyFile(error.kind()))?);
        let mut key = Zeroizing::new([0_u8; KEY_LEN]);
        // The decode error is dropped unread: it can quote a byte of the key.
        let decoded = STANDARD
            .decode_slice(text.trim_ascii(), key.as_mut_slice())
            .map_err(|_| Error::KeyFormat)?;
        if decoded != KEY_LEN {
            return Err(Error::KeyFormat);
        }
        Ok(Self::from_key(key))
    }

    /// Builds a vault from raw key bytes: the core of `from_key_file`, and the test entry
    /// point that skips writing a key file.
    pub(super) fn from_key(key: Zeroizing<[u8; KEY_LEN]>) -> Self {
        let key_id = key_id(&key);
        Self { key, key_id }
    }

    /// Seals a secret with a fresh random nonce, bound to `context` (for example a device
    /// and credential kind; may be empty), so the value opens only with the same context.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] when the OS random source yields no nonce.
    ///
    /// # Panics
    ///
    /// Only if the secret exceeds the cipher's 256 GiB limit, which no secret held in memory
    /// reaches.
    pub fn seal(&self, plaintext: &Secret, context: &[u8]) -> Result<Sealed, Error> {
        let mut nonce = XNonce::default();
        getrandom::fill(&mut nonce).map_err(Error::Random)?;
        let mut buffer = Zeroizing::new(Vec::with_capacity(plaintext.expose().len() + TAG_LEN));
        buffer.extend_from_slice(plaintext.expose());
        self.cipher()
            .encrypt_in_place(
                &nonce,
                &associated_data(&self.key_id, context),
                &mut *buffer,
            )
            .expect("a secret held in memory is far below the cipher's length limit");
        Ok(Sealed {
            nonce: nonce.into(),
            ciphertext: std::mem::take(&mut *buffer),
            key_id: self.key_id.clone(),
        })
    }

    /// Opens a value sealed by [`Vault::seal`] with the same `context`.
    ///
    /// # Errors
    ///
    /// [`Error::WrongKey`] when another key sealed it, [`Error::Decrypt`] when authentication
    /// fails (changed nonce, ciphertext or key id, a different key, or another context).
    pub fn open(&self, sealed: &Sealed, context: &[u8]) -> Result<Secret, Error> {
        if sealed.key_id != self.key_id {
            return Err(Error::WrongKey);
        }
        let mut buffer = Zeroizing::new(sealed.ciphertext.clone());
        self.cipher()
            .decrypt_in_place(
                &XNonce::from(sealed.nonce),
                &associated_data(&sealed.key_id, context),
                &mut *buffer,
            )
            .map_err(|_| Error::Decrypt)?;
        Ok(Secret::new(std::mem::take(&mut *buffer)))
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new((&*self.key).into())
    }
}

impl fmt::Debug for Vault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vault")
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

/// Names a key without revealing it: the first 8 bytes of SHA-256(domain ‖ key) in lowercase hex.
fn key_id(key: &[u8; KEY_LEN]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::new()
        .chain_update(KEY_ID_DOMAIN)
        .chain_update(key)
        .finalize();
    let mut id = String::with_capacity(KEY_ID_BYTES * 2);
    for byte in &digest[..KEY_ID_BYTES] {
        id.push(char::from(HEX[usize::from(byte >> 4)]));
        id.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    id
}

/// Format tag ‖ key id ‖ context, each prefixed with its length as a big-endian `u64`,
/// so no two different inputs encode to the same bytes.
fn associated_data(key_id: &str, context: &[u8]) -> Vec<u8> {
    let parts = [SEALED_TAG, key_id.as_bytes(), context];
    let mut aad = Vec::with_capacity(parts.iter().map(|part| 8 + part.len()).sum());
    for part in parts {
        aad.extend_from_slice(&(part.len() as u64).to_be_bytes());
        aad.extend_from_slice(part);
    }
    aad
}

/// A sealed secret: nonce, ciphertext (with tag) and the id of the key that sealed it.
///
/// Holds no plaintext, so it may be printed, copied and compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    nonce: [u8; NONCE_LEN],
    ciphertext: Vec<u8>,
    key_id: String,
}

impl Sealed {
    /// Rebuilds a sealed value from stored columns.
    ///
    /// # Errors
    ///
    /// [`Error::Malformed`] when the nonce is not 24 bytes, the ciphertext is shorter than
    /// the tag, or the key id is empty or longer than 255 bytes.
    pub fn from_parts(nonce: &[u8], ciphertext: Vec<u8>, key_id: String) -> Result<Self, Error> {
        let nonce = <[u8; NONCE_LEN]>::try_from(nonce).map_err(|_| Error::Malformed)?;
        if ciphertext.len() < TAG_LEN || key_id.is_empty() || key_id.len() > usize::from(u8::MAX) {
            return Err(Error::Malformed);
        }
        Ok(Self {
            nonce,
            ciphertext,
            key_id,
        })
    }

    #[must_use]
    pub fn nonce(&self) -> &[u8] {
        &self.nonce
    }

    #[must_use]
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// One-column layout: `0x01 ‖ len(key_id) ‖ key_id ‖ nonce ‖ ciphertext`.
    ///
    /// # Panics
    ///
    /// Never: [`Sealed::from_parts`] and [`Vault`] keep key ids within 255 bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let id = self.key_id.as_bytes();
        let mut bytes = Vec::with_capacity(2 + id.len() + NONCE_LEN + self.ciphertext.len());
        bytes.push(LAYOUT_V1);
        bytes.push(u8::try_from(id.len()).expect("key ids are at most 255 bytes"));
        bytes.extend_from_slice(id);
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.ciphertext);
        bytes
    }

    /// Parses the layout written by [`Sealed::to_bytes`]; lengths are checked, never trusted.
    ///
    /// # Errors
    ///
    /// [`Error::Malformed`] for an unknown version, a truncated value or a key id that is not UTF-8.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let (&version, rest) = bytes.split_first().ok_or(Error::Malformed)?;
        if version != LAYOUT_V1 {
            return Err(Error::Malformed);
        }
        let (&id_len, rest) = rest.split_first().ok_or(Error::Malformed)?;
        let (id, rest) = rest
            .split_at_checked(usize::from(id_len))
            .ok_or(Error::Malformed)?;
        let (nonce, ciphertext) = rest.split_at_checked(NONCE_LEN).ok_or(Error::Malformed)?;
        let id = std::str::from_utf8(id).map_err(|_| Error::Malformed)?;
        Self::from_parts(nonce, ciphertext.to_vec(), id.to_owned())
    }
}
