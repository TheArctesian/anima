//! The user-based security model's cryptography for SNMP v3 authPriv (RFC 3414, RFC 7860,
//! RFC 3826): password to key, key localisation, HMAC-SHA-256-192 and AES-128-CFB. Pure:
//! the salt comes in as a parameter.
//!
//! Every key and every buffer that held a password is zeroized when it drops.

use aes::{
    Aes128,
    cipher::{Array, BlockCipherEncrypt as _},
};
use hmac::{Hmac, KeyInit, Mac as _};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

/// How many bytes of the repeated password are hashed (RFC 3414 A.2.2).
const EXPANDED: usize = 1024 * 1024;
/// The length of `usmHMAC192SHA256AuthProtocol` parameters (RFC 7860 Section 4.2.1).
pub(in crate::checks) const DIGEST_LEN: usize = 24;
/// The length of the AES salt carried in `msgPrivacyParameters` (RFC 3826 Section 3.1.2.1).
pub(in crate::checks) const SALT_LEN: usize = 8;

/// A 32-byte SHA-256 key, wiped on drop.
pub(in crate::checks) type Key = Zeroizing<[u8; 32]>;

/// The password's key `Ku`: SHA-256 over the password repeated to 1 MiB (RFC 3414 A.2.2 with
/// SHA-256, RFC 7860 Section 4.2.2). `None` for an empty password, which has no key.
#[must_use]
pub(in crate::checks) fn password_key(password: &[u8]) -> Option<Key> {
    if password.is_empty() {
        return None;
    }
    let mut hasher = Sha256::new();
    let mut chunk = Zeroizing::new([0_u8; 64]);
    let mut index = 0;
    for _ in 0..EXPANDED / chunk.len() {
        for byte in chunk.iter_mut() {
            *byte = password[index % password.len()];
            index += 1;
        }
        hasher.update(chunk.as_slice());
    }
    let mut key = Zeroizing::new([0_u8; 32]);
    key.copy_from_slice(&hasher.finalize());
    Some(key)
}

/// The key localised to the engine: SHA-256(Ku || engineID || Ku) (RFC 3414 A.2.2).
#[must_use]
pub(in crate::checks) fn localise(key: &Key, engine_id: &[u8]) -> Key {
    let mut hasher = Sha256::new();
    hasher.update(key.as_slice());
    hasher.update(engine_id);
    hasher.update(key.as_slice());
    let mut local = Zeroizing::new([0_u8; 32]);
    local.copy_from_slice(&hasher.finalize());
    local
}

/// The localised keys of one engine for one user.
pub(in crate::checks) struct Keys {
    pub(in crate::checks) auth: Key,
    pub(in crate::checks) privacy: Key,
}

impl Keys {
    /// Both keys of the passwords, localised to the engine. `None` when a password is empty.
    #[must_use]
    pub(in crate::checks) fn localised(auth: &[u8], privacy: &[u8], engine_id: &[u8]) -> Option<Self> {
        Some(Self {
            auth: localise(&password_key(auth)?, engine_id),
            privacy: localise(&password_key(privacy)?, engine_id),
        })
    }
}

fn mac(key: &Key) -> Hmac<Sha256> {
    Hmac::<Sha256>::new_from_slice(key.as_slice()).expect("HMAC takes a key of any length")
}

/// The first 24 bytes of HMAC-SHA-256 over the whole message, whose authentication
/// parameters are 24 zero bytes (RFC 7860 Section 4.2.4).
#[must_use]
pub(in crate::checks) fn digest(key: &Key, message: &[u8]) -> [u8; DIGEST_LEN] {
    let mut mac = mac(key);
    mac.update(message);
    let full = mac.finalize().into_bytes();
    let mut out = [0_u8; DIGEST_LEN];
    out.copy_from_slice(&full[..DIGEST_LEN]);
    out
}

/// Whether `received` is the digest of the message, compared in constant time.
#[must_use]
pub(in crate::checks) fn verify(key: &Key, message: &[u8], received: &[u8]) -> bool {
    if received.len() != DIGEST_LEN {
        return false;
    }
    let mut mac = mac(key);
    mac.update(message);
    mac.verify_truncated_left(received).is_ok()
}

/// The AES-128-CFB IV: engine boots, engine time and the salt (RFC 3826 Section 3.1.2.1).
#[must_use]
pub(in crate::checks) fn iv(boots: u32, time: u32, salt: [u8; SALT_LEN]) -> [u8; 16] {
    let mut iv = [0_u8; 16];
    iv[..4].copy_from_slice(&boots.to_be_bytes());
    iv[4..8].copy_from_slice(&time.to_be_bytes());
    iv[8..].copy_from_slice(&salt);
    iv
}

/// Whether CFB feeds back the ciphertext it writes or the ciphertext it reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Encrypt,
    Decrypt,
}

/// AES-128 in CFB with 128-bit segments over `data`, in place. The key is the first 16 bytes
/// of the localised privacy key (RFC 3826 Section 3.1.2.1).
fn cfb(key: &Key, iv: [u8; 16], data: &mut [u8], direction: Direction) {
    let mut cipher_key = Zeroizing::new([0_u8; 16]);
    cipher_key.copy_from_slice(&key[..16]);
    let cipher = <Aes128 as KeyInit>::new(&Array::from(*cipher_key));
    let mut register = iv;
    for block in data.chunks_mut(16) {
        let mut stream = Array::from(register);
        cipher.encrypt_block(&mut stream);
        let mut next = [0_u8; 16];
        for (index, byte) in block.iter_mut().enumerate() {
            let input = *byte;
            *byte ^= stream[index];
            next[index] = if direction == Direction::Encrypt { *byte } else { input };
        }
        register = next;
    }
}

/// Encrypts the scoped PDU in place.
pub(in crate::checks) fn encrypt(key: &Key, iv: [u8; 16], data: &mut [u8]) {
    cfb(key, iv, data, Direction::Encrypt);
}

/// Decrypts the scoped PDU in place.
pub(in crate::checks) fn decrypt(key: &Key, iv: [u8; 16], data: &mut [u8]) {
    cfb(key, iv, data, Direction::Decrypt);
}
