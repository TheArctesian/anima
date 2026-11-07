//! D-5 and H-4: the hub's fingerprint form, parsed and shown the same way.

use crate::web_view::{Fingerprint, FingerprintError};

/// Every hex digit, in both nibbles of a byte.
const HEX: &str = "0123456789abcdeffedcba98765432100f1e2d3c4b5a69788796a5b4c3d2e1f0";

fn with_colons(hex: &str) -> String {
    hex.as_bytes()
        .chunks(2)
        .map(|pair| std::str::from_utf8(pair).unwrap())
        .collect::<Vec<_>>()
        .join(":")
}

fn parse(text: &str) -> Result<Fingerprint, FingerprintError> {
    text.parse()
}

#[test]
fn d_5_reads_64_hex_digits_with_or_without_colons_in_any_case() {
    let mixed: String = HEX
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i % 3 == 0 {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect();
    for text in [
        HEX.to_owned(),
        HEX.to_uppercase(),
        mixed.clone(),
        with_colons(HEX),
        with_colons(&HEX.to_uppercase()),
        with_colons(&mixed),
    ] {
        assert_eq!(parse(&text).unwrap().to_string(), HEX, "{text}");
    }
}

#[test]
fn d_5_forms_of_one_fingerprint_are_equal_and_other_digits_are_not() {
    let other = format!("1{}", &HEX[1..]);
    assert_eq!(parse(HEX), parse(&with_colons(&HEX.to_uppercase())));
    assert_ne!(parse(HEX).unwrap(), parse(&other).unwrap());
}

#[test]
fn d_5_shows_each_byte_as_two_lower_case_digits() {
    let low = "00".repeat(31) + "0a";
    assert_eq!(parse(&low).unwrap().to_string(), low);
    let high = "ff".repeat(32);
    assert_eq!(parse(&high.to_uppercase()).unwrap().to_string(), high);
}

#[test]
fn d_5_refuses_every_other_form() {
    let bad = [
        String::new(),
        ":".to_owned(),
        HEX[..62].to_owned(),
        HEX[..63].to_owned(),
        format!("{HEX}0"),
        format!("{HEX}00"),
        format!(" {HEX}"),
        format!("{HEX}\n"),
        format!("SHA256:{HEX}"),
        HEX.replace('0', "g"),
        HEX.replacen('0', "+", 1),
        HEX.replacen("01", "+1", 1),
        HEX.replacen('0', "é", 1),
        HEX.replacen("01", "é", 1),
        with_colons(HEX).replace(':', "-"),
        with_colons(HEX).replace(':', " "),
        format!("{}:", with_colons(HEX)),
        format!(":{}", with_colons(HEX)),
        with_colons(&HEX[..62]),
        format!("{}:{}", with_colons(HEX), "00"),
        format!("{}:{}", &HEX[..2], &HEX[2..]),
        format!("{}:{}", &HEX[..3], &HEX[3..]),
        with_colons(HEX).replacen(':', "::", 1),
        with_colons(HEX).replacen(':', "", 1),
        // 32 groups and 64 digits, but not every group is a pair.
        with_colons(HEX).replacen("01:23", "012:3", 1),
    ];
    for text in bad {
        assert_eq!(parse(&text), Err(FingerprintError), "{text:?}");
    }
}

#[test]
fn log_007_the_fingerprint_error_carries_only_a_code() {
    let error = parse(&format!("{HEX}0")).unwrap_err();
    assert_eq!(FingerprintError::CODE, "fingerprint");
    assert!(!error.to_string().contains(HEX));
}

#[test]
fn web_008_the_fingerprint_of_a_certificate_is_the_sha_256_of_its_der() {
    assert_eq!(
        Fingerprint::of_certificate(b"").to_string(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        Fingerprint::of_certificate(b"abc").to_string(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
