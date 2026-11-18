//! The stream form of a sealed piece (K-E4 Section 9, FIL-002). Pure: bytes in, bytes out.
//!
//! ```text
//! piece  = 0x01 ‖ len(key id) ‖ key id ‖ part ‖ part ‖ …     (at least one part)
//! part   = nonce (24) ‖ ciphertext ‖ tag (16)                 (64 KiB of plain bytes, the last 0 to 64 KiB)
//! aad    = tag text ‖ key name ‖ key id ‖ binding ‖ part number ‖ last-part mark
//! ```
//!
//! The binding is the hash of the plain bytes, so a part opens only in its own piece; the part
//! number stops a move of a part, and the last-part mark stops a cut at a part boundary.

use chacha20poly1305::{XNonce, aead::AeadInOut};
use zeroize::Zeroizing;

use crate::engine::sealing_keys::SealingKey;

/// Plain bytes in one part.
pub const PART: usize = 64 * 1024;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
/// The bytes of one full part as stored.
pub const RECORD: usize = NONCE_LEN + PART + TAG_LEN;
const LAYOUT_V1: u8 = 0x01;
const PART_TAG: &[u8] = b"anima/files/part/v1";

/// Why a piece does not open. Holds no byte of the piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StreamError {
    #[error("the piece was sealed with a different key")]
    WrongKey,
    #[error("the piece is cut, changed or a different piece")]
    Damaged,
}

/// What a part is sealed for: one key and one piece.
#[derive(Clone, Copy)]
pub struct Binding<'a> {
    pub key: &'a SealingKey,
    /// The hash of the plain bytes in lower-case hexadecimal, or the text of a staging copy.
    pub piece: &'a str,
}

impl Binding<'_> {
    /// The header that starts the piece: the layout and the identifier of the key.
    ///
    /// # Panics
    /// Never: a key identifier is 16 bytes.
    #[must_use]
    pub fn header(&self) -> Vec<u8> {
        let id = self.key.id().as_bytes();
        let mut header = Vec::with_capacity(2 + id.len());
        header.push(LAYOUT_V1);
        header.push(u8::try_from(id.len()).expect("a key identifier is 16 bytes"));
        header.extend_from_slice(id);
        header
    }

    /// The length of the header that `bytes` starts with, when it names this key.
    ///
    /// # Errors
    /// `WrongKey` for the header of a different key; `Damaged` for a header that is cut or of
    /// an unknown layout.
    pub fn header_len(&self, bytes: &[u8]) -> Result<usize, StreamError> {
        let [version, id_len, rest @ ..] = bytes else {
            return Err(StreamError::Damaged);
        };
        if *version != LAYOUT_V1 {
            return Err(StreamError::Damaged);
        }
        let id = rest
            .get(..usize::from(*id_len))
            .ok_or(StreamError::Damaged)?;
        if id != self.key.id().as_bytes() {
            return Err(StreamError::WrongKey);
        }
        Ok(2 + id.len())
    }

    /// One part: a fresh nonce, then `plain` sealed for its number and mark.
    ///
    /// # Panics
    /// When the OS gives no random nonce, or for more than [`PART`] plain bytes.
    #[must_use]
    pub fn seal_part(&self, number: u64, last: bool, plain: &[u8]) -> Vec<u8> {
        assert!(plain.len() <= PART, "a part holds at most 64 KiB");
        let mut nonce = XNonce::default();
        getrandom::fill(&mut nonce).expect("the OS random source gives a nonce");
        let mut record = Vec::with_capacity(NONCE_LEN + plain.len() + TAG_LEN);
        record.extend_from_slice(&nonce);
        let mut sealed = Zeroizing::new(plain.to_vec());
        self.key
            .cipher()
            .encrypt_in_place(&nonce, &self.associated(number, last), &mut *sealed)
            .expect("a part is far below the cipher's length limit");
        record.extend_from_slice(&sealed);
        record
    }

    /// The plain bytes of one part, sealed for this number and mark.
    ///
    /// # Errors
    /// `Damaged` when the part is cut, changed, of another piece, number or mark.
    pub fn open_part(
        &self,
        number: u64,
        last: bool,
        record: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, StreamError> {
        if record.len() < NONCE_LEN + TAG_LEN || record.len() > RECORD {
            return Err(StreamError::Damaged);
        }
        let (nonce, sealed) = record.split_at(NONCE_LEN);
        let nonce = <[u8; NONCE_LEN]>::try_from(nonce).map_err(|_| StreamError::Damaged)?;
        let mut plain = Zeroizing::new(sealed.to_vec());
        self.key
            .cipher()
            .decrypt_in_place(
                &XNonce::from(nonce),
                &self.associated(number, last),
                &mut *plain,
            )
            .map_err(|_| StreamError::Damaged)?;
        Ok(plain)
    }

    fn associated(&self, number: u64, last: bool) -> Vec<u8> {
        let number = number.to_be_bytes();
        let mark = [u8::from(last)];
        let parts: [&[u8]; 6] = [
            PART_TAG,
            self.key.name().as_bytes(),
            self.key.id().as_bytes(),
            self.piece.as_bytes(),
            &number,
            &mark,
        ];
        let mut aad = Vec::with_capacity(parts.iter().map(|part| 8 + part.len()).sum());
        for part in parts {
            aad.extend_from_slice(&(part.len() as u64).to_be_bytes());
            aad.extend_from_slice(part);
        }
        aad
    }
}

/// The whole piece of `plain`, for the tests; the hub seals as a stream.
#[cfg(test)]
#[must_use]
pub fn seal_all(binding: Binding<'_>, plain: &[u8]) -> Vec<u8> {
    let mut piece = binding.header();
    let mut parts = plain.chunks(PART).peekable();
    if parts.peek().is_none() {
        piece.extend(binding.seal_part(0, true, &[]));
    }
    let mut number = 0;
    while let Some(part) = parts.next() {
        piece.extend(binding.seal_part(number, parts.peek().is_none(), part));
        number += 1;
    }
    piece
}

/// The plain bytes of a whole piece, read as the stream reader reads it: a part is the last
/// one exactly when no byte follows it.
///
/// # Errors
/// As [`Binding::header_len`] and [`Binding::open_part`]; `Damaged` for a piece with no part.
#[cfg(test)]
pub fn open_all(binding: Binding<'_>, piece: &[u8]) -> Result<Vec<u8>, StreamError> {
    let mut rest = &piece[binding.header_len(piece)?..];
    if rest.is_empty() {
        return Err(StreamError::Damaged);
    }
    let mut plain = Vec::new();
    let mut number = 0;
    while !rest.is_empty() {
        let (record, after) = rest.split_at(rest.len().min(RECORD));
        plain.extend_from_slice(&binding.open_part(number, after.is_empty(), record)?);
        rest = after;
        number += 1;
    }
    Ok(plain)
}
