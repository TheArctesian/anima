//! The SHA-256 fingerprint of a leaf certificate, in the hub's form (D-5, H-4).

use std::{fmt, str::FromStr};

use sha2::{Digest, Sha256};

const BYTES: usize = 32;

/// SHA-256 of a leaf certificate's DER. Shown as 64 lower-case hex digits, the hub's form.
///
/// A fingerprint is public, so plain equality is enough; it is no secret to compare in
/// constant time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint([u8; BYTES]);

impl Fingerprint {
    /// The fingerprint of a DER-encoded certificate.
    pub fn of_certificate(der: &[u8]) -> Self {
        Self(Sha256::digest(der).into())
    }
}

/// The text is not 64 hex digits, with or without colons between every byte pair.
/// Carries no part of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the fingerprint is not 64 hexadecimal digits")]
pub struct FingerprintError;

impl FingerprintError {
    /// Stable code the client switches on.
    pub const CODE: &'static str = "fingerprint";
}

/// 64 hex digits in any case; colons either between every byte pair or not at all (H-4).
impl FromStr for Fingerprint {
    type Err = FingerprintError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits: Vec<u8> = if text.contains(':') {
            let pairs: Vec<&[u8]> = text.split(':').map(str::as_bytes).collect();
            if pairs.len() != BYTES || pairs.iter().any(|pair| pair.len() != 2) {
                return Err(FingerprintError);
            }
            pairs.concat()
        } else {
            text.as_bytes().to_vec()
        };
        if digits.len() != BYTES * 2 {
            return Err(FingerprintError);
        }
        let mut bytes = [0; BYTES];
        for (byte, pair) in bytes.iter_mut().zip(digits.as_chunks::<2>().0) {
            *byte = (hex_value(pair[0])? << 4) | hex_value(pair[1])?;
        }
        Ok(Self(bytes))
    }
}

/// The value of one ASCII hex digit; no sign, no other byte.
fn hex_value(digit: u8) -> Result<u8, FingerprintError> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'a'..=b'f' => Ok(digit - b'a' + 10),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err(FingerprintError),
    }
}

/// 64 lower-case hex digits, no separators.
impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

/// Serialises as the hub's form, 64 lower-case hex digits (D-5).
impl serde::Serialize for Fingerprint {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
