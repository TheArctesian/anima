//! A database login's password: 192 random bits as 48 hexadecimal characters.
//!
//! It has no `Debug`, `Display` or `Clone`, and its text is wiped when it drops (LOG-007).
//! Hexadecimal needs no quoting in SQL, in a URL or in an `initdb` password file.

use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

const BYTES: usize = 24;

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

    /// The text to give to PostgreSQL. Never log it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}
