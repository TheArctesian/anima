//! Secret bytes that are wiped on drop and cannot be printed, copied or serialised.

use zeroize::Zeroizing;

/// Plaintext credential bytes: zeroized on drop.
///
/// Deliberately implements no `Debug`, `Display`, `Serialize`, `Clone` or `PartialEq`
/// (LOG-007, SEC-005): it cannot reach a log, an API response, a stray copy or a
/// timing-dependent comparison.
pub struct Secret(Zeroizing<Vec<u8>>);

impl Secret {
    /// Takes ownership of the bytes; they are zeroized when the secret drops.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// The plaintext bytes. Callers must not log or keep copies of them.
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl From<String> for Secret {
    /// Reuses the string's buffer, so no unzeroized copy is left behind.
    fn from(text: String) -> Self {
        Self::new(text.into_bytes())
    }
}
