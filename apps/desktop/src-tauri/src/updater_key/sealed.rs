//! The updater key as the OS key store keeps it: minisign's own scrypt-sealed key bytes, never
//! the bare seed (BLD-045, D-19). Any process of the operator's user can read the key store
//! (Secret Service has no per-app access), so what it finds there is useless without the
//! password. Pure: the salt is a parameter (CQ-021), and nothing here reads a file or a store.

use base64::{Engine, engine::general_purpose::STANDARD};
use blake2::{Blake2b256, Digest};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use super::{
    error::UpdaterKeyError,
    secret::{
        CHK_ALG, CHK_BYTES, KDF_SCRYPT, KEY_BYTES, MEM_AT, OPS_AT, PASSWORD_MAX_BYTES, SALT_AT,
        SALT_BYTES, SEALED_AT, SEALED_BYTES, SIG_ALG, SecretKey, key_bytes,
    },
};

/// minisign's ceiling: scrypt `N` at most 2^20 (1 GiB with r = 8). Real key files use p = 1, so
/// a crafted file cannot ask for more parallel work than that.
const N_LOG2_MAX: u8 = 20;
/// A key file sealed with a smaller scrypt `N` than 2^15 (minisign's default) is sealed anew
/// on import (D-19).
const N_LOG2_MIN: u8 = 15;
/// The fewest characters a password the app seals a key with may have (D-19).
const NEW_PASSWORD_MIN_CHARS: usize = 12;
const P_MAX: u32 = 1;
/// The limits a key is sealed with here: minisign's own defaults, scrypt N = 2^15, r = 8, p = 1.
const SEAL_OPSLIMIT: u64 = 1_048_576;
const SEAL_MEMLIMIT: u64 = 33_554_432;

/// The key and the text to keep in the key store, from an imported key file (BLD-045).
pub(crate) struct Imported {
    pub key: SecretKey,
    pub stored: Zeroizing<String>,
}

/// Whether the checksum is BLAKE2b-256 of the algorithm, key number and secret key.
pub(super) fn checksum_holds(bytes: &[u8]) -> bool {
    let sealed = &bytes[SEALED_AT..];
    let (body, stored) = sealed.split_at(SEALED_BYTES - CHK_BYTES);
    checksum(body).as_slice().ct_eq(stored).into()
}

fn checksum(body: &[u8]) -> [u8; CHK_BYTES] {
    let mut hash = Blake2b256::new();
    hash.update(SIG_ALG);
    hash.update(body);
    hash.finalize().into()
}

/// The scrypt `log2(N)` and `p` the key bytes ask for; `None` above the ceilings.
fn cost_of(bytes: &[u8]) -> Option<(u8, u32)> {
    let ops = u64::from_le_bytes(bytes[OPS_AT..MEM_AT].try_into().ok()?);
    let mem = u64::from_le_bytes(bytes[MEM_AT..SEALED_AT].try_into().ok()?);
    scrypt_cost(ops, mem)
}

/// XORs the sealed part with the scrypt stream of `password`, which opens or seals it.
pub(super) fn unseal(bytes: &mut [u8], password: &[u8]) -> Result<(), UpdaterKeyError> {
    let (n_log2, p) = cost_of(bytes).ok_or(UpdaterKeyError::Format)?;
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
/// and the minisign crate compute them (r is 8). `None` above the ceilings.
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

/// A password that can seal or open a key: not empty, and not absurdly long.
fn usable(password: Option<&str>) -> Option<&str> {
    password.filter(|text| !text.is_empty() && text.len() <= PASSWORD_MAX_BYTES)
}

/// A password to seal a key with anew: given, and at least `NEW_PASSWORD_MIN_CHARS` long.
fn new_password_of(new_password: Option<&str>) -> Result<&str, UpdaterKeyError> {
    let new = usable(new_password).ok_or(UpdaterKeyError::NewPasswordNeeded)?;
    if new.chars().count() < NEW_PASSWORD_MIN_CHARS {
        return Err(UpdaterKeyError::PasswordTooShort);
    }
    Ok(new)
}

/// `key` sealed with `password` under `salt`, as base64 of minisign's key bytes.
pub(super) fn seal(
    key: &SecretKey,
    password: &str,
    salt: &[u8; SALT_BYTES],
) -> Result<Zeroizing<String>, UpdaterKeyError> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(KEY_BYTES));
    bytes.extend_from_slice(SIG_ALG);
    bytes.extend_from_slice(KDF_SCRYPT);
    bytes.extend_from_slice(CHK_ALG);
    bytes.extend_from_slice(salt);
    bytes.extend_from_slice(&SEAL_OPSLIMIT.to_le_bytes());
    bytes.extend_from_slice(&SEAL_MEMLIMIT.to_le_bytes());
    let body = key.sealed_body();
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(&checksum(&body));
    unseal(&mut bytes, password.as_bytes())?;
    Ok(Zeroizing::new(STANDARD.encode(bytes.as_slice())))
}

/// What to store for an imported key file (BLD-045, D-19). A file sealed with a password is kept
/// as it is, once `password` opens it, unless its scrypt `N` is below 2^15: then it is sealed
/// anew under `salt`, with `new_password` when one is given and with `password` when not. A file
/// with no password, or an empty one, is sealed anew with `new_password`; without one it is
/// `NewPasswordNeeded`. A new password shorter than 12 characters is `PasswordTooShort`.
pub(crate) fn imported(
    file: &[u8],
    password: Option<&str>,
    new_password: Option<&str>,
    salt: &[u8; SALT_BYTES],
) -> Result<Imported, UpdaterKeyError> {
    let bytes = key_bytes(file)?;
    if let (true, Some(given)) = (&bytes[2..4] == KDF_SCRYPT, usable(password)) {
        let key = open(&bytes, given)?;
        if cost_of(&bytes).is_some_and(|(n_log2, _)| n_log2 >= N_LOG2_MIN) {
            let stored = Zeroizing::new(STANDARD.encode(bytes.as_slice()));
            return Ok(Imported { key, stored });
        }
        let reseal = match usable(new_password) {
            Some(_) => new_password_of(new_password)?,
            None => given,
        };
        let stored = seal(&key, reseal, salt)?;
        return Ok(Imported { key, stored });
    }
    let key = SecretKey::from_key_file(file, None)?;
    let new = new_password_of(new_password)?;
    let stored = seal(&key, new, salt)?;
    Ok(Imported { key, stored })
}

/// Opens sealed key bytes with `password`; a checksum that fails is `WrongPassword`.
fn open(bytes: &[u8], password: &str) -> Result<SecretKey, UpdaterKeyError> {
    let mut opened = Zeroizing::new(bytes.to_vec());
    unseal(&mut opened, password.as_bytes())?;
    if !checksum_holds(&opened) {
        return Err(UpdaterKeyError::WrongPassword);
    }
    SecretKey::from_sealed(&opened[SEALED_AT..])
}

/// Opens what `imported` stored (BLD-045). Only a sealed key is accepted: a bare key in the store
/// is `Format`, never used. A key whose public key is not `public`, the line stored beside it, is
/// `Mismatch` (D-19).
pub(crate) fn unlocked(
    stored: &str,
    password: &str,
    public: &str,
) -> Result<SecretKey, UpdaterKeyError> {
    let password = usable(Some(password)).ok_or(UpdaterKeyError::WrongPassword)?;
    let bytes = Zeroizing::new(
        STANDARD
            .decode(stored.trim())
            .map_err(|_| UpdaterKeyError::Format)?,
    );
    let shaped = bytes.len() == KEY_BYTES
        && &bytes[..2] == SIG_ALG
        && &bytes[2..4] == KDF_SCRYPT
        && &bytes[4..SALT_AT] == CHK_ALG;
    if !shaped {
        return Err(UpdaterKeyError::Format);
    }
    let key = open(&bytes, password)?;
    if key.public_key().line() != public.trim() {
        return Err(UpdaterKeyError::Mismatch);
    }
    Ok(key)
}
