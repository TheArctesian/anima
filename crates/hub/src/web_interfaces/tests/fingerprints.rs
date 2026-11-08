//! WEB-008 and D-5: the fingerprint form.

use crate::web_interfaces::rules::fingerprint;

const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn with_colons(hex: &str) -> String {
    hex.as_bytes()
        .chunks(2)
        .map(|pair| std::str::from_utf8(pair).unwrap())
        .collect::<Vec<_>>()
        .join(":")
}

#[test]
fn d_5_accepts_64_hex_digits_with_or_without_colons_in_any_case() {
    for text in [
        HEX.to_owned(),
        HEX.to_uppercase(),
        with_colons(HEX),
        with_colons(&HEX.to_uppercase()),
    ] {
        assert_eq!(fingerprint(&text).unwrap().hex(), HEX, "{text}");
    }
}

#[test]
fn d_5_refuses_every_other_form_on_the_fingerprint_field() {
    let mut bad = vec![
        String::new(),
        HEX[..62].to_owned(),
        format!("{HEX}0"),
        format!(" {HEX}"),
        format!("{HEX}\n"),
        format!("SHA256:{HEX}"),
        HEX.replace('0', "g"),
        HEX.replacen('0', "é", 1),
        with_colons(HEX).replace(':', "-"),
        format!("{}:", with_colons(HEX)),
        with_colons(&HEX[..62]),
        format!("{}:{}", &HEX[..3], &HEX[3..]),
    ];
    bad.push(with_colons(HEX).replacen(':', "::", 1));
    for text in bad {
        let error = fingerprint(&text).unwrap_err();
        assert_eq!(error.field, "fingerprintSha256", "{text:?}");
        assert!(!error.message.contains(&text) || text.is_empty());
    }
}
