//! The answers of the two sign-in methods that use a password. Pure: the random bytes are
//! arguments.
//!
//! The big numbers of `num-bigint` cannot be zeroed: the secret number and the shared
//! secret stay in freed memory until it is used again. The byte copies are zeroed.

use aes::cipher::{BlockCipherEncrypt as _, KeyInit as _};
use md5::{Digest as _, Md5};
use num_bigint::BigUint;
use zeroize::Zeroizing;

/// The longest account name or password Apple's method carries, in bytes.
pub(super) const APPLE_LONGEST: usize = 63;
/// The lengths of a Diffie-Hellman number taken from a device, in bytes: 1024 bits, what
/// macOS sends, to 4096 bits. A shorter number lets anyone who sees the exchange find the
/// secret and read the password; a longer one costs seconds of arithmetic.
pub(super) const KEY_BYTES: std::ops::RangeInclusive<usize> = 128..=512;

/// VNC authentication (RFC 6143 Section 7.2.2): the challenge encrypted with DES under the
/// password. The password is cut to 8 bytes and each key byte has its bits in reverse
/// order, as every VNC server expects.
pub(super) fn vnc_password_answer(challenge: [u8; 16], password: &str) -> [u8; 16] {
    let mut key = Zeroizing::new([0u8; 8]);
    for (slot, byte) in key.iter_mut().zip(password.bytes()) {
        *slot = byte.reverse_bits();
    }
    let cipher = des::Des::new(&(*key).into());
    let mut answer = challenge;
    let (blocks, _) = answer.as_chunks_mut::<8>();
    for block in blocks {
        cipher.encrypt_block(block.into());
    }
    answer
}

/// What a device sends to open Apple's method (security type 30).
pub(super) struct AppleChallenge {
    pub(super) generator: u16,
    pub(super) prime: Vec<u8>,
    pub(super) server_public: Vec<u8>,
}

/// Why Apple's method cannot be answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoAppleAnswer {
    /// The account name or the password is longer than the method carries.
    TooLong,
    /// The device's numbers are not a usable key exchange.
    BadNumbers,
}

/// The answer to Apple's method: the account name and password, each in 64 bytes ending
/// with a zero byte, encrypted with AES-128-ECB under the MD5 of the Diffie-Hellman
/// secret, then the client's public number. `private` is the client's secret number and
/// `filler` fills the unused bytes of the two fields; both must be random.
pub(super) fn apple_answer(
    challenge: &AppleChallenge,
    (username, password): (&str, &str),
    private: &[u8],
    filler: [u8; 128],
) -> Result<Vec<u8>, NoAppleAnswer> {
    if username.len() > APPLE_LONGEST || password.len() > APPLE_LONGEST {
        return Err(NoAppleAnswer::TooLong);
    }
    let length = challenge.prime.len();
    let prime = BigUint::from_bytes_be(&challenge.prime);
    let server_public = BigUint::from_bytes_be(&challenge.server_public);
    let one = BigUint::from(1u8);
    // The prime must really have its stated length and be odd; a public number of 0, 1 or
    // prime - 1 would make the secret a number anyone knows.
    let usable = KEY_BYTES.contains(&length)
        && prime.bits() > u64::try_from((length - 1) * 8).unwrap_or(u64::MAX)
        && prime.bit(0)
        && challenge.generator > 1
        && server_public > one
        && server_public < &prime - &one;
    if !usable {
        return Err(NoAppleAnswer::BadNumbers);
    }
    let private = BigUint::from_bytes_be(private);
    let public = BigUint::from(challenge.generator).modpow(&private, &prime);
    let secret = Zeroizing::new(padded(&server_public.modpow(&private, &prime), length));
    let key: [u8; 16] = Md5::digest(&*secret).into();
    let key = Zeroizing::new(key);

    let mut fields = Zeroizing::new(filler);
    put(&mut fields[..64], username.as_bytes());
    put(&mut fields[64..], password.as_bytes());
    let cipher = aes::Aes128::new(&(*key).into());
    let (blocks, _) = fields.as_chunks_mut::<16>();
    for block in blocks {
        cipher.encrypt_block(block.into());
    }
    let mut answer = fields.to_vec();
    answer.extend_from_slice(&padded(&public, length));
    Ok(answer)
}

/// `text` and a zero byte at the start of `field`. `text` is shorter than `field`.
fn put(field: &mut [u8], text: &[u8]) {
    field[..text.len()].copy_from_slice(text);
    field[text.len()] = 0;
}

/// The big-endian bytes of `number`, with zeros in front up to `length` bytes.
fn padded(number: &BigUint, length: usize) -> Vec<u8> {
    let bytes = Zeroizing::new(number.to_bytes_be());
    let mut out = vec![0; length.saturating_sub(bytes.len())];
    out.extend_from_slice(&bytes);
    out
}
