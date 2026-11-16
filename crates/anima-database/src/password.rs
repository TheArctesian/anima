//! A database login's password: 192 random bits as 48 hexadecimal characters.
//!
//! It has no `Debug`, `Display` or `Clone`, and its text is wiped when it drops (LOG-007).
//! Hexadecimal needs no quoting in a URL.
//!
//! PostgreSQL is never sent the password to store. It is sent the SCRAM verifier, which is
//! what it would store itself. So no statement holds a password, and none can reach a log,
//! the activity view or a statistics extension, whatever their settings are.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

const BYTES: usize = 24;

/// PostgreSQL's own count for a new verifier (`scram_iterations`).
const ITERATIONS: u32 = 4096;

type HmacSha256 = Hmac<Sha256>;

pub struct Password(Zeroizing<String>);

impl Password {
    /// A new password from the operating system's random source.
    ///
    /// # Errors
    /// When the operating system gives no random bytes.
    pub fn random() -> Result<Self, getrandom::Error> {
        let mut bytes = Zeroizing::new([0_u8; BYTES]);
        getrandom::fill(bytes.as_mut_slice())?;
        Ok(Self::from_bytes(&bytes))
    }

    /// The password that `bytes` spell in lower-case hexadecimal.
    pub(crate) fn from_bytes(bytes: &[u8; BYTES]) -> Self {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut text = Zeroizing::new(String::with_capacity(BYTES * 2));
        for byte in bytes {
            text.push(char::from(DIGITS[usize::from(byte >> 4)]));
            text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        Self(text)
    }

    /// A password that was read back from where this crate stored it.
    pub(crate) fn from_text(text: String) -> Self {
        Self(Zeroizing::new(text))
    }

    /// The `SCRAM-SHA-256` verifier of this password with `salt` (RFC 5803), in the form
    /// PostgreSQL keeps in `pg_authid` and takes in `CREATE ROLE ... PASSWORD`.
    pub(crate) fn scram_verifier(&self, salt: &[u8; 16]) -> String {
        let salted = Zeroizing::new(salted_password(self.0.as_bytes(), salt));
        let client_key = Zeroizing::new(keyed(salted.as_slice(), b"Client Key"));
        let stored_key = Sha256::digest(client_key.as_slice());
        let server_key = keyed(salted.as_slice(), b"Server Key");
        format!(
            "SCRAM-SHA-256${ITERATIONS}:{}${}:{}",
            STANDARD.encode(salt),
            STANDARD.encode(stored_key),
            STANDARD.encode(server_key),
        )
    }

    /// The text for the URL file and the sign-in. Never log it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

fn keyed(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC takes a key of any length");
    mac.update(message);
    mac.finalize().into_bytes().into()
}

/// PBKDF2 with HMAC-SHA-256 for one block of output, which is all that SCRAM-SHA-256 uses.
fn salted_password(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut first = HmacSha256::new_from_slice(password).expect("HMAC takes a key of any length");
    first.update(salt);
    first.update(&1_u32.to_be_bytes());
    let mut block: [u8; 32] = first.finalize().into_bytes().into();
    let mut output = block;
    for _ in 1..ITERATIONS {
        block = keyed(password, &block);
        for (byte, next) in output.iter_mut().zip(block) {
            *byte ^= next;
        }
    }
    output
}
