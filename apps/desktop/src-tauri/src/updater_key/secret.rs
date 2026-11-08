//! A minisign secret key, read from the key file `tauri signer generate` writes and decrypted in
//! memory (BLD-045). Pure: no file, no key store, no log.
//!
//! The key file is the minisign text (an untrusted comment line, then base64 of the key) or, as
//! Tauri writes it, base64 of that text. The key bytes are, in order: signature algorithm `Ed`,
//! KDF algorithm (`Sc` for scrypt or two zero bytes), checksum algorithm `B2`, the scrypt salt,
//! opslimit and memlimit, then the sealed part: key number, Ed25519 secret key (seed then public
//! key) and a `BLAKE2b-256` checksum. With `Sc` the sealed part is XOR-ed with an `scrypt` stream.

use base64::{Engine, engine::general_purpose::STANDARD};
use blake2::{Blake2b256, Digest};
use ed25519_dalek::SigningKey;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use super::error::UpdaterKeyError;

const SIG_ALG: &[u8] = b"Ed";
const KDF_SCRYPT: &[u8] = b"Sc";
const KDF_NONE: &[u8] = &[0, 0];
const CHK_ALG: &[u8] = b"B2";
const COMMENT_PREFIX: &str = "untrusted comment: ";

const KEYNUM_BYTES: usize = 8;
const SEED_BYTES: usize = 32;
const PUBLIC_BYTES: usize = 32;
const SALT_BYTES: usize = 32;
const CHK_BYTES: usize = 32;
/// Where the salt, the two limits and the sealed part start in the key bytes.
const SALT_AT: usize = 6;
const OPS_AT: usize = SALT_AT + SALT_BYTES;
const MEM_AT: usize = OPS_AT + 8;
const SEALED_AT: usize = MEM_AT + 8;
const SEALED_BYTES: usize = KEYNUM_BYTES + SEED_BYTES + PUBLIC_BYTES + CHK_BYTES;
const KEY_BYTES: usize = SEALED_AT + SEALED_BYTES;

/// minisign's own ceilings: scrypt `N` at most 2^20 (1 GiB with r = 8). `p` is capped too, so a
/// crafted file cannot ask for hours of work; real key files use p = 1.
const N_LOG2_MAX: u8 = 20;
const P_MAX: u32 = 16;
const PASSWORD_MAX_BYTES: usize = 1024;

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

    /// The text kept in the OS key store: base64 of the key number, the seed and the public key.
    pub(crate) fn to_stored(&self) -> Zeroizing<String> {
        let mut bytes = Zeroizing::new(Vec::with_capacity(SEALED_BYTES));
        bytes.extend_from_slice(&self.keynum);
        bytes.extend_from_slice(self.seed.as_slice());
        bytes.extend_from_slice(&self.public);
        Zeroizing::new(STANDARD.encode(bytes.as_slice()))
    }

    /// Reads what `to_stored` wrote. The public key must match the seed.
    pub(crate) fn from_stored(text: &str) -> Result<Self, UpdaterKeyError> {
        let bytes = Zeroizing::new(
            STANDARD
                .decode(text.trim())
                .map_err(|_| UpdaterKeyError::Format)?,
        );
        if bytes.len() != KEYNUM_BYTES + SEED_BYTES + PUBLIC_BYTES {
            return Err(UpdaterKeyError::Format);
        }
        Self::from_sealed(&bytes)
    }

    /// Key number, seed and public key; the public key must be the seed's.
    fn from_sealed(sealed: &[u8]) -> Result<Self, UpdaterKeyError> {
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

    /// The Ed25519 signing key; it wipes itself when dropped.
    pub(crate) fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.seed)
    }
}

/// The 158 key bytes of a key file in either form. Lengths are checked, never trusted.
fn key_bytes(file: &[u8]) -> Result<Zeroizing<Vec<u8>>, UpdaterKeyError> {
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

/// Whether the checksum is BLAKE2b-256 of the algorithm, key number and secret key.
fn checksum_holds(bytes: &[u8]) -> bool {
    let sealed = &bytes[SEALED_AT..];
    let (body, stored) = sealed.split_at(SEALED_BYTES - CHK_BYTES);
    let mut hash = Blake2b256::new();
    hash.update(SIG_ALG);
    hash.update(body);
    let computed = hash.finalize();
    computed.as_slice().ct_eq(stored).into()
}

/// XORs the sealed part with the scrypt stream of `password`, which opens it.
fn unseal(bytes: &mut [u8], password: &[u8]) -> Result<(), UpdaterKeyError> {
    let ops = u64::from_le_bytes(bytes[OPS_AT..MEM_AT].try_into().unwrap_or_default());
    let mem = u64::from_le_bytes(bytes[MEM_AT..SEALED_AT].try_into().unwrap_or_default());
    let (n_log2, p) = scrypt_cost(ops, mem).ok_or(UpdaterKeyError::Format)?;
    let params = scrypt::Params::new(n_log2, 8, p).map_err(|_| UpdaterKeyError::Format)?;
    let mut stream = Zeroizing::new([0; SEALED_BYTES]);
    scrypt::scrypt(
        password,
        &bytes[SALT_AT..OPS_AT],
        &params,
        stream.as_mut_slice(),
    )
    .map_err(|_| UpdaterKeyError::Format)?;
    for (byte, mask) in bytes[SEALED_AT..].iter_mut().zip(stream.iter()) {
        *byte ^= mask;
    }
    Ok(())
}

/// scrypt `log2(N)` and `p` from minisign's opslimit and memlimit, as libsodium's `pickparams`
/// and the minisign crate compute them (r is 8). `None` above minisign's ceilings.
pub(super) fn scrypt_cost(opslimit: u64, memlimit: u64) -> Option<(u8, u32)> {
    const R: u64 = 8;
    let ops = opslimit.max(32_768);
    let smallest_above = |max_n: u64| {
        let mut n_log2 = 1_u8;
        while n_log2 < 63 && (1_u64 << n_log2) <= max_n / 2 {
            n_log2 += 1;
        }
        n_log2
    };
    let (n_log2, p) = if ops < memlimit / 32 {
        (smallest_above(ops / (R * 4)), 1)
    } else {
        let n_log2 = smallest_above(memlimit / (R * 128));
        let rp = ((ops / 4) >> n_log2).min(0x3fff_ffff);
        (n_log2, u32::try_from(rp / R).ok()?)
    };
    (n_log2 <= N_LOG2_MAX && (1..=P_MAX).contains(&p)).then_some((n_log2, p))
}
