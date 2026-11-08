//! A certificate's SHA-256 fingerprint (SEC-007, WEB-008, D-5): one value, read from 64 hex
//! digits or from 32 hex pairs joined by colons, in either case, and written in the form each
//! store keeps. Pure.

use std::str::FromStr;

use sha2::{Digest, Sha256};

const BYTES: usize = 32;

/// The SHA-256 of a certificate's DER. Two texts that name the same digest, in either form
/// or case, are one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CertificateFingerprint([u8; BYTES]);

impl CertificateFingerprint {
    /// The fingerprint of a certificate's DER.
    #[must_use]
    pub fn of(der: &[u8]) -> Self {
        Self(Sha256::digest(der).into())
    }

    /// 32 lower-case hex pairs joined by colons, as `FreeRDP` writes it and `trust` stores it
    /// (`server_certificates_fingerprint_check`).
    #[must_use]
    pub fn colon_pairs(&self) -> String {
        self.rendered(":")
    }

    /// 64 lower-case hex digits, as web interfaces store it (`pins_fingerprint_check`, D-5).
    #[must_use]
    pub fn hex(&self) -> String {
        self.rendered("")
    }

    /// The bytes as lower-case hex pairs with `separator` between them.
    fn rendered(&self, separator: &str) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut text = String::with_capacity(BYTES * (2 + separator.len()));
        for (index, byte) in self.0.iter().enumerate() {
            if index > 0 {
                text.push_str(separator);
            }
            text.push(char::from(DIGITS[usize::from(byte >> 4)]));
            text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        text
    }
}

/// The text is neither 64 hex digits nor 32 hex pairs joined by colons. It holds no part of
/// the text (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a SHA-256 certificate fingerprint")]
pub struct BadCertificateFingerprint;

impl FromStr for CertificateFingerprint {
    type Err = BadCertificateFingerprint;

    /// Either form, any case. Nothing else is allowed, not even surrounding space: a caller
    /// that allows it trims first.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.as_bytes();
        let pairs: Vec<&[u8]> = if text.contains(&b':') {
            text.split(|&byte| byte == b':').collect()
        } else {
            text.chunks(2).collect()
        };
        if pairs.len() != BYTES {
            return Err(BadCertificateFingerprint);
        }
        let mut bytes = [0; BYTES];
        for (byte, pair) in bytes.iter_mut().zip(pairs) {
            let &[high, low] = pair else {
                return Err(BadCertificateFingerprint);
            };
            *byte = (nibble(high)? << 4) | nibble(low)?;
        }
        Ok(Self(bytes))
    }
}

/// The value of one ASCII hex digit.
fn nibble(digit: u8) -> Result<u8, BadCertificateFingerprint> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'a'..=b'f' => Ok(digit - b'a' + 10),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err(BadCertificateFingerprint),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{BadCertificateFingerprint, CertificateFingerprint};

    /// SHA-256 of no bytes, in each rendering.
    const PAIRS: &str = "e3:b0:c4:42:98:fc:1c:14:9a:fb:f4:c8:99:6f:b9:24:\
                         27:ae:41:e4:64:9b:93:4c:a4:95:99:1b:78:52:b8:55";
    const HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn parse(text: &str) -> Result<CertificateFingerprint, BadCertificateFingerprint> {
        text.parse()
    }

    #[test]
    fn q2_10_of_is_the_sha256_of_the_bytes_in_both_renderings() {
        let empty = CertificateFingerprint::of(b"");
        assert_eq!(empty.colon_pairs(), PAIRS);
        assert_eq!(empty.hex(), HEX);
        assert_ne!(CertificateFingerprint::of(b"x"), empty);
    }

    #[test]
    fn q2_10_both_forms_in_either_case_parse_to_one_value() {
        let empty = CertificateFingerprint::of(b"");
        for text in [
            PAIRS.to_owned(),
            PAIRS.to_ascii_uppercase(),
            HEX.to_owned(),
            HEX.to_ascii_uppercase(),
        ] {
            assert_eq!(parse(&text), Ok(empty), "{text}");
        }
        let all = "0123456789abcdefABCDEF0123456789abcdef0123456789abcdef0123456789";
        assert_eq!(
            parse(all).unwrap().hex(),
            "0123456789abcdefabcdef0123456789abcdef0123456789abcdef0123456789",
            "every digit has its value"
        );
    }

    #[test]
    fn q2_10_any_other_text_is_refused() {
        let mixed = format!("{}{}", &PAIRS[..48], &HEX[32..]);
        let refused = [
            String::new(),
            format!(" {HEX}"),
            format!("{PAIRS}\n"),
            HEX[..62].to_owned(),
            format!("{HEX}0"),
            format!("{HEX}00"),
            HEX.replacen('e', "g", 1),
            HEX.replacen('0', "é", 1),
            PAIRS.replace(':', "-"),
            format!("{PAIRS}:"),
            format!("{PAIRS}:00"),
            PAIRS[..PAIRS.len() - 3].to_owned(),
            PAIRS.replacen(':', "::", 1),
            PAIRS.replacen("e3:b0", "e:3b0", 1),
            format!("{}:{}", &HEX[..3], &HEX[3..]),
            format!("SHA256:{HEX}"),
            "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk".to_owned(),
            mixed,
        ];
        for text in refused {
            assert_eq!(parse(&text), Err(BadCertificateFingerprint), "{text:?}");
        }
    }

    proptest! {
        #[test]
        fn q2_10_each_rendering_parses_back_to_its_value(der in any::<Vec<u8>>()) {
            let fingerprint = CertificateFingerprint::of(&der);
            prop_assert_eq!(parse(&fingerprint.colon_pairs()), Ok(fingerprint));
            prop_assert_eq!(parse(&fingerprint.hex()), Ok(fingerprint));
            prop_assert_eq!(fingerprint.colon_pairs().replace(':', ""), fingerprint.hex());
        }

        #[test]
        fn q2_10_any_text_parses_without_panic_and_what_parses_keeps_its_digits(
            text in prop_oneof![any::<String>(), "[0-9a-fA-F:]{62,96}"]
        ) {
            if let Ok(fingerprint) = parse(&text) {
                prop_assert_eq!(fingerprint.hex(), text.replace(':', "").to_ascii_lowercase());
            }
        }
    }
}
