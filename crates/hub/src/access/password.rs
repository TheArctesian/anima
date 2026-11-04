//! Operator passwords: argon2id hashes in PHC form, checked in constant time (SEC-002, D-8).
//!
//! Pure: the salt is a parameter, so the caller owns the randomness. The work is CPU-bound;
//! callers run it off the async executor.

use argon2::{Argon2, PasswordHasher as _, PasswordVerifier as _, password_hash};
use zeroize::Zeroizing;

/// Salt length in bytes (the PHC recommendation).
pub(super) const SALT_LEN: usize = 16;

/// Longest password accepted, in bytes. Longer input is refused, not truncated.
pub(super) const MAX_PASSWORD_LEN: usize = 1024;

/// Shortest password `operator set` accepts, in characters.
pub(super) const MIN_PASSWORD_CHARS: usize = 12;

/// A hash of a password nobody knows, with the same parameters as [`hash`]. An unknown
/// username is checked against it, so the response time does not reveal whether the
/// username exists (D-8).
pub(super) const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$\
    QAPHY+9ozYD2ngwK8LPQBw$gMCoS2TIjT8q83WoOYF7B+AeFyEmZe5PSde/QkkiXAM";

/// A password as typed: wiped from memory on drop, never printed (LOG-007).
pub struct Password(Zeroizing<String>);

impl Password {
    #[must_use]
    pub fn new(text: String) -> Self {
        Self(Zeroizing::new(text))
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub(super) fn chars(&self) -> usize {
        self.0.chars().count()
    }
}

/// The stored hash could not be read, or hashing failed.
#[derive(Debug, thiserror::Error)]
#[error("the password hash cannot be used")]
pub struct HashError;

impl From<password_hash::Error> for HashError {
    fn from(_: password_hash::Error) -> Self {
        Self
    }
}

/// argon2id, version 0x13, m = 19 MiB, t = 2, p = 1: the OWASP minimum, which is also the
/// crate default.
fn hasher() -> Argon2<'static> {
    Argon2::default()
}

/// The PHC string of `password` with `salt`.
pub(super) fn hash(password: &Password, salt: &[u8; SALT_LEN]) -> Result<String, HashError> {
    let phc = hasher().hash_password_with_salt(password.as_bytes(), salt)?;
    Ok(phc.to_string())
}

/// Whether `password` matches the PHC string `phc`. The comparison is constant-time inside
/// the crate. A password longer than [`MAX_PASSWORD_LEN`] still runs one verification, so
/// its length does not shorten the response.
pub(super) fn matches(password: &Password, phc: &str) -> Result<bool, HashError> {
    let too_long = password.as_bytes().len() > MAX_PASSWORD_LEN;
    let candidate = if too_long {
        &[][..]
    } else {
        password.as_bytes()
    };
    match hasher().verify_password(candidate, phc) {
        Ok(()) => Ok(!too_long),
        Err(password_hash::Error::PasswordInvalid) => Ok(false),
        Err(error) => Err(error.into()),
    }
}
