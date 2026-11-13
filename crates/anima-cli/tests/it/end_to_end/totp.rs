//! The authenticator app of the test: RFC 6238 codes from the seed the hub gave, written here
//! apart from the hub's own code.

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, KeyInit as _, Mac as _};
use sha1::Sha1;

/// The `secret` parameter of an `otpauth://` URI, base32-decoded (RFC 4648).
pub fn secret_of(uri: &str) -> Vec<u8> {
    let encoded = uri
        .split(['?', '&'])
        .find_map(|part| part.strip_prefix("secret="))
        .expect("a secret parameter");
    let (mut buffer, mut bits, mut out) = (0_u32, 0, Vec::new());
    for c in encoded.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'2'..=b'7' => c - b'2' + 26,
            _ => panic!("not base32: {c}"),
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).expect("one byte"));
        }
    }
    out
}

/// The six-digit code for `secret` now (HMAC-SHA-1, 30 s steps).
pub fn code_now(secret: &[u8]) -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("any key length");
    mac.update(&(seconds / 30).to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[19] & 0x0f);
    let word = u32::from_be_bytes(digest[offset..offset + 4].try_into().expect("four bytes"));
    format!("{:06}", (word & 0x7fff_ffff) % 1_000_000)
}

/// A six-digit code that is not `right`.
pub fn wrong_code(right: &str) -> String {
    let first = if right.starts_with('0') { '1' } else { '0' };
    format!("{first}{}", &right[1..])
}
