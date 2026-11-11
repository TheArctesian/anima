//! One STREAM part (K4 §11, G-2): STREAM BE32 built by hand on `XChaCha20Poly1305`, because
//! `aead` 0.6 has no `stream` module.
//!
//! The nonce of part `n` is the header's 19-byte prefix ‖ `n` as a big-endian `u32` ‖ the last
//! flag (`0x01` on the last part, `0x00` before it). The associated data of every part is the
//! whole header, so a part opens only in its own file, at its own place, with its own flag.

use chacha20poly1305::{XNonce, aead::AeadInOut as _};
use zeroize::Zeroizing;

use super::{
    Damage, LENGTH_LEN, MAX_PART, TAG_LEN,
    header::{HEADER_LEN, NONCE_PREFIX_LEN},
    key::RecordingKey,
};

/// Where a part sits in the stream: its counter and whether it is the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Place {
    pub(super) counter: u32,
    pub(super) last: bool,
}

/// The nonce of the part at `place`.
pub(super) fn nonce(prefix: &[u8; NONCE_PREFIX_LEN], place: Place) -> XNonce {
    let mut nonce = XNonce::default();
    let (head, tail) = nonce.split_at_mut(NONCE_PREFIX_LEN);
    head.copy_from_slice(prefix);
    tail[..4].copy_from_slice(&place.counter.to_be_bytes());
    tail[4] = u8::from(place.last);
    nonce
}

/// The nonce prefix inside encoded header bytes: their last 19 bytes.
fn prefix_of(header: &[u8; HEADER_LEN]) -> &[u8; NONCE_PREFIX_LEN] {
    header[HEADER_LEN - NONCE_PREFIX_LEN..]
        .try_into()
        .expect("the prefix ends the header")
}

/// Seals `plaintext` (at most [`MAX_PART`] bytes) as the part at `place`, framed: the 4-byte
/// big-endian ciphertext length, then the ciphertext and its tag.
pub(super) fn seal(
    key: &RecordingKey,
    header: &[u8; HEADER_LEN],
    place: Place,
    plaintext: &[u8],
) -> Vec<u8> {
    debug_assert!(plaintext.len() <= MAX_PART, "the sealer bounds a part");
    let length = u32::try_from(plaintext.len() + TAG_LEN).expect("a part is at most 64 KiB");
    let mut framed = Vec::with_capacity(LENGTH_LEN + plaintext.len() + TAG_LEN);
    framed.extend_from_slice(&length.to_be_bytes());
    framed.extend_from_slice(plaintext);
    let tag = key
        .cipher()
        .encrypt_inout_detached(
            &nonce(prefix_of(header), place),
            header,
            (&mut framed[LENGTH_LEN..]).into(),
        )
        .expect("a part of at most 64 KiB is far below the cipher's limit");
    framed.extend_from_slice(&tag);
    framed
}

/// Opens the part at `counter`, trying it as a part before the last, then as the last.
/// Returns the plaintext and whether it was the last part.
///
/// A tag check fails before any byte is decrypted, so the second try sees the ciphertext
/// unchanged.
pub(super) fn open(
    key: &RecordingKey,
    header: &[u8; HEADER_LEN],
    counter: u32,
    ciphertext: Vec<u8>,
) -> Result<(Zeroizing<Vec<u8>>, bool), Damage> {
    let cipher = key.cipher();
    let mut buffer = Zeroizing::new(ciphertext);
    for last in [false, true] {
        let nonce = nonce(prefix_of(header), Place { counter, last });
        if cipher
            .decrypt_in_place(&nonce, header, &mut *buffer)
            .is_ok()
        {
            return Ok((buffer, last));
        }
    }
    Err(Damage::Tag)
}
