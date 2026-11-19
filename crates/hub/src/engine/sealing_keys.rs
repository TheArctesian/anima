//! The data keys: one key for each extension and engine part, which seals its records and its
//! files on the hub (K-E5 Section 1; VLT-003, R-146). A leaf: it imports no domain.
//!
//! A data key is never the credential key: at start [`SealingKeys::open`] compares the identifier
//! of each key with the identifier of the credential key and with each other, and refuses two
//! that are equal. No error text and no `Debug` output holds a key or a plain value (LOG-007).

use std::{
    fmt,
    fs::{DirBuilder, OpenOptions},
    io::{self, Write as _},
    os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce, aead::AeadInOut};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
/// The longest key name, in characters.
const MAX_NAME: usize = 40;
const KEY_ID_BYTES: usize = 8;
/// The same domain as the credential key's identifier, so that equal bytes give equal
/// identifiers and the start check can compare them (VLT-003).
const KEY_ID_DOMAIN: &[u8] = b"anima/credentials/key-id/v1";
const SEALED_TAG: &[u8] = b"anima/data/sealed/v1";
const LAYOUT_V1: u8 = 0x01;
const KEY_SUFFIX: &str = ".key";

/// Why a data key cannot be used. No variant holds a key or a plain value (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum SealingKeyError {
    /// The hub has no `ANIMA_DATA_KEY_DIR`: a domain that seals answers `UNAVAILABLE`.
    #[error("the hub has no folder of data keys (ANIMA_DATA_KEY_DIR)")]
    Unavailable,
    #[error("a data key name is 1 to 40 lower-case letters, digits or underscores")]
    BadName,
    #[error("the folder of data keys cannot be read or made ({0})")]
    Folder(io::ErrorKind),
    #[error("the data key {name} cannot be read or written ({kind})")]
    KeyFile { name: String, kind: io::ErrorKind },
    #[error("the data key {name} is not 32 bytes in base64")]
    KeyFormat { name: String },
    #[error("the data key {name} is the credential key (VLT-003)")]
    CredentialKey { name: String },
    #[error("the data keys {first} and {second} are the same key (VLT-003)")]
    SameKey { first: String, second: String },
    #[error("the value was sealed with a different key")]
    WrongKey,
    #[error("the sealed value is damaged or has a different context")]
    Damaged,
}

/// The folder of data keys; `None` when the hub has none.
#[derive(Clone, Debug)]
pub struct SealingKeys {
    folder: Option<PathBuf>,
}

impl SealingKeys {
    /// No data keys: a hub with no `ANIMA_DATA_KEY_DIR`.
    #[must_use]
    pub const fn none() -> Self {
        Self { folder: None }
    }

    /// The data keys of `folder`, checked: each key file that exists must hold a key, and no
    /// key may equal the credential key (`credential_key_id`, the text of
    /// `Vault::key_id`) or another data key (VLT-003). Makes the folder (mode 0700) when it
    /// does not exist. With no folder the hub has no data keys.
    ///
    /// # Errors
    /// `Folder`, `KeyFile`, `KeyFormat`, `CredentialKey`, `SameKey`.
    pub fn open(folder: Option<&Path>, credential_key_id: &[u8]) -> Result<Self, SealingKeyError> {
        let Some(folder) = folder else {
            return Ok(Self { folder: None });
        };
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(folder)
            .map_err(|error| SealingKeyError::Folder(error.kind()))?;
        let mut seen: Vec<(String, String)> = Vec::new();
        for name in key_names(folder)? {
            let key = read_key(folder, &name)?;
            if key.id.as_bytes() == credential_key_id {
                return Err(SealingKeyError::CredentialKey { name });
            }
            if let Some((first, _)) = seen.iter().find(|(_, id)| *id == key.id) {
                return Err(SealingKeyError::SameKey {
                    first: first.clone(),
                    second: name,
                });
            }
            seen.push((name, key.id.clone()));
        }
        Ok(Self {
            folder: Some(folder.to_path_buf()),
        })
    }

    /// Whether the hub has a folder of data keys.
    #[must_use]
    pub const fn available(&self) -> bool {
        self.folder.is_some()
    }

    /// The key `name`. A key with no file is made now (32 random bytes, mode 0600); a file that
    /// exists is never written again.
    ///
    /// # Errors
    /// `Unavailable` with no folder; `BadName`; `KeyFile`, `KeyFormat` for a file that cannot
    /// be read or made.
    pub fn key(&self, name: &str) -> Result<SealingKey, SealingKeyError> {
        let folder = self.folder.as_deref().ok_or(SealingKeyError::Unavailable)?;
        if !is_name(name) {
            return Err(SealingKeyError::BadName);
        }
        match read_key(folder, name) {
            Err(SealingKeyError::KeyFile {
                kind: io::ErrorKind::NotFound,
                ..
            }) => {
                make_key(folder, name)?;
                read_key(folder, name)
            }
            read => read,
        }
    }
}

/// One data key: 32 bytes, zeroed on drop. `Debug` shows the name and the identifier only.
pub struct SealingKey {
    name: String,
    key: Zeroizing<[u8; KEY_LEN]>,
    id: String,
}

impl SealingKey {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The identifier of the key, not the key: the first 8 bytes of a SHA-256, in hexadecimal.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The cipher of this key, for the stream form of a file (K-E4 Section 9).
    #[must_use]
    pub fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new((&*self.key).into())
    }

    /// Seals `plain` for `context` (for a record `<schema>.<table>:<id>`): the layout of the
    /// credential vault, `0x01 ‖ len(id) ‖ id ‖ nonce ‖ ciphertext`.
    ///
    /// # Panics
    /// When the OS gives no random nonce, which a running hub does not see.
    #[must_use]
    pub fn seal(&self, context: &str, plain: &[u8]) -> Vec<u8> {
        let mut nonce = XNonce::default();
        getrandom::fill(&mut nonce).expect("the OS random source gives a nonce");
        let mut buffer = Zeroizing::new(Vec::with_capacity(plain.len() + TAG_LEN));
        buffer.extend_from_slice(plain);
        self.cipher()
            .encrypt_in_place(&nonce, &self.associated(context), &mut *buffer)
            .expect("a value held in memory is far below the cipher's length limit");
        let id = self.id.as_bytes();
        let mut sealed = Vec::with_capacity(2 + id.len() + NONCE_LEN + buffer.len());
        sealed.push(LAYOUT_V1);
        sealed.push(u8::try_from(id.len()).expect("a key identifier is 16 bytes"));
        sealed.extend_from_slice(id);
        sealed.extend_from_slice(&nonce);
        sealed.extend_from_slice(&buffer);
        sealed
    }

    /// Opens a value that [`SealingKey::seal`] sealed for the same `context`. Lengths are checked,
    /// never trusted.
    ///
    /// # Errors
    /// `WrongKey` when a different key sealed it; `Damaged` for a value that is cut, changed or
    /// of a different context.
    pub fn open(&self, context: &str, sealed: &[u8]) -> Result<Vec<u8>, SealingKeyError> {
        let (&version, rest) = sealed.split_first().ok_or(SealingKeyError::Damaged)?;
        let (&id_len, rest) = rest.split_first().ok_or(SealingKeyError::Damaged)?;
        if version != LAYOUT_V1 {
            return Err(SealingKeyError::Damaged);
        }
        let (id, rest) = rest
            .split_at_checked(usize::from(id_len))
            .ok_or(SealingKeyError::Damaged)?;
        if id != self.id.as_bytes() {
            return Err(SealingKeyError::WrongKey);
        }
        let (nonce, ciphertext) = rest
            .split_at_checked(NONCE_LEN)
            .ok_or(SealingKeyError::Damaged)?;
        let nonce = <[u8; NONCE_LEN]>::try_from(nonce).map_err(|_| SealingKeyError::Damaged)?;
        let mut buffer = ciphertext.to_vec();
        self.cipher()
            .decrypt_in_place(&XNonce::from(nonce), &self.associated(context), &mut buffer)
            .map_err(|_| SealingKeyError::Damaged)?;
        Ok(buffer)
    }

    /// Tag ‖ key name ‖ key identifier ‖ context, each after its length as a big-endian `u64`.
    fn associated(&self, context: &str) -> Vec<u8> {
        let parts = [
            SEALED_TAG,
            self.name.as_bytes(),
            self.id.as_bytes(),
            context.as_bytes(),
        ];
        let mut aad = Vec::with_capacity(parts.iter().map(|part| 8 + part.len()).sum());
        for part in parts {
            aad.extend_from_slice(&(part.len() as u64).to_be_bytes());
            aad.extend_from_slice(part);
        }
        aad
    }
}

impl fmt::Debug for SealingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SealingKey")
            .field("name", &self.name)
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Whether `name` can name a data key: `^[a-z][a-z0-9_]{0,39}$`. Thus it is never a path.
#[must_use]
pub fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && name.len() <= MAX_NAME
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The names of the key files in `folder`, sorted. Other files are not keys and are left.
fn key_names(folder: &Path) -> Result<Vec<String>, SealingKeyError> {
    let entries =
        std::fs::read_dir(folder).map_err(|error| SealingKeyError::Folder(error.kind()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| SealingKeyError::Folder(error.kind()))?;
        let file = entry.file_name();
        let name = file.to_str().and_then(|file| file.strip_suffix(KEY_SUFFIX));
        if let Some(name) = name.filter(|name| is_name(name)) {
            names.push(name.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

fn key_path(folder: &Path, name: &str) -> PathBuf {
    folder.join(format!("{name}{KEY_SUFFIX}"))
}

fn read_key(folder: &Path, name: &str) -> Result<SealingKey, SealingKeyError> {
    let unreadable = |error: io::Error| SealingKeyError::KeyFile {
        name: name.to_owned(),
        kind: error.kind(),
    };
    let text = Zeroizing::new(std::fs::read(key_path(folder, name)).map_err(unreadable)?);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    // The decode error is dropped unread: it can quote a byte of the key.
    let decoded = STANDARD.decode_slice(text.trim_ascii(), key.as_mut_slice());
    if decoded.ok() != Some(KEY_LEN) {
        return Err(SealingKeyError::KeyFormat {
            name: name.to_owned(),
        });
    }
    let id = key_id(&key);
    Ok(SealingKey {
        name: name.to_owned(),
        key,
        id,
    })
}

/// Writes a new key file that only the hub user can read: first under a temporary name, then
/// linked to its name, so that no reader sees a file that is half written. A file that another
/// request made first is kept: the caller reads it.
fn make_key(folder: &Path, name: &str) -> Result<(), SealingKeyError> {
    let failed = |error: io::Error| SealingKeyError::KeyFile {
        name: name.to_owned(),
        kind: error.kind(),
    };
    let mut random = Zeroizing::new([0_u8; KEY_LEN + 8]);
    getrandom::fill(random.as_mut_slice()).map_err(|_| SealingKeyError::KeyFile {
        name: name.to_owned(),
        kind: io::ErrorKind::Other,
    })?;
    let (key, marker) = random.split_at(KEY_LEN);
    let text = Zeroizing::new(format!("{}\n", STANDARD.encode(key)));
    let temporary = folder.join(format!(".{name}.{}.tmp", hex(marker)));
    let written = write_private(&temporary, text.as_bytes())
        .and_then(|()| std::fs::hard_link(&temporary, key_path(folder, name)));
    let removed = std::fs::remove_file(&temporary);
    match written {
        Ok(()) => removed.map_err(failed),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(failed(error)),
    }
}

/// A new file with mode 0600 that holds `bytes`, flushed to the disk.
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Names a key without showing it: the first 8 bytes of SHA-256(domain ‖ key), in hexadecimal,
/// as the credential vault names its key.
fn key_id(key: &[u8; KEY_LEN]) -> String {
    let digest = Sha256::new()
        .chain_update(KEY_ID_DOMAIN)
        .chain_update(key)
        .finalize();
    hex(&digest[..KEY_ID_BYTES])
}

/// `bytes` in lower-case hexadecimal.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    text
}
