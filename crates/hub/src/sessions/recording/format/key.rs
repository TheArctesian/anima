//! A recording's key (K4 §11, SEC-003): 32 random bytes, one key per recording.

use chacha20poly1305::{KeyInit as _, XChaCha20Poly1305};
use zeroize::{Zeroize as _, Zeroizing};

/// Bytes in a recording key.
pub const KEY_LEN: usize = 32;

/// A recording's key. Wiped on drop. Implements no `Debug`, `Display`, `Clone` or `Serialize`,
/// so it cannot reach a log or an error (LOG-007).
pub struct RecordingKey(Zeroizing<[u8; KEY_LEN]>);

impl RecordingKey {
    /// Takes the key bytes (fresh OS randomness for a new recording), then wipes the copy it
    /// was given.
    pub fn from_bytes(mut bytes: [u8; KEY_LEN]) -> Self {
        let key = Self(Zeroizing::new(bytes));
        bytes.zeroize();
        key
    }

    /// The cipher for this key; it wipes its own copy of the key on drop.
    pub(super) fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new((&*self.0).into())
    }
}
