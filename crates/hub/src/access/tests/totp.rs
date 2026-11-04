//! The RFC 6238 core: test vectors, the drift window, step reuse and the enrolment URI
//! (SEC-002).

use chrono::{DateTime, TimeZone, Utc};

use crate::access::totp::{accepted_step, context, otpauth_uri, step_at};

/// The SHA-1 secret of RFC 6238 Appendix B.
const RFC_SECRET: &[u8] = b"12345678901234567890";

fn at(unix: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(unix, 0).unwrap()
}

/// RFC 6238 Appendix B, SHA-1 rows, cut to the low six digits (RFC 4226 truncation modulo
/// 10^6 is the low six digits of the eight-digit value).
const VECTORS: [(i64, &str); 6] = [
    (59, "287082"),
    (1_111_111_109, "081804"),
    (1_111_111_111, "050471"),
    (1_234_567_890, "005924"),
    (2_000_000_000, "279037"),
    (20_000_000_000, "353130"),
];

#[test]
fn sec_002_rfc_6238_sha1_vectors_are_accepted_at_their_step() {
    for (unix, code) in VECTORS {
        let step = step_at(at(unix));
        assert_eq!(step, unix / 30, "{unix}");
        assert_eq!(
            accepted_step(RFC_SECRET, code, at(unix), None),
            Some(step),
            "{unix} {code}"
        );
    }
}

#[test]
fn sec_002_rfc_6238_vectors_fail_with_another_secret_or_digit() {
    for (unix, code) in VECTORS {
        assert_eq!(
            accepted_step(b"12345678901234567891", code, at(unix), None),
            None
        );
        let mut changed = code.as_bytes().to_vec();
        changed[5] = if changed[5] == b'9' {
            b'0'
        } else {
            changed[5] + 1
        };
        let changed = String::from_utf8(changed).unwrap();
        assert_eq!(accepted_step(RFC_SECRET, &changed, at(unix), None), None);
    }
}

/// `050471` is the code of step 37037037, which runs from 1111111110 to 1111111139.
const STEP: i64 = 37_037_037;
const CODE: &str = "050471";

#[test]
fn sec_002_one_step_of_drift_is_accepted_either_side() {
    let earliest = (STEP - 1) * 30;
    let latest = (STEP + 2) * 30 - 1;
    for unix in [earliest, STEP * 30, STEP * 30 + 29, latest] {
        assert_eq!(
            accepted_step(RFC_SECRET, CODE, at(unix), None),
            Some(STEP),
            "{unix}"
        );
    }
}

#[test]
fn sec_002_two_steps_of_drift_are_refused() {
    for unix in [(STEP - 1) * 30 - 1, (STEP + 2) * 30] {
        assert_eq!(
            accepted_step(RFC_SECRET, CODE, at(unix), None),
            None,
            "{unix}"
        );
    }
}

#[test]
fn sec_002_a_used_step_or_an_earlier_one_is_refused() {
    let now = at(STEP * 30);
    assert_eq!(accepted_step(RFC_SECRET, CODE, now, Some(STEP)), None);
    assert_eq!(accepted_step(RFC_SECRET, CODE, now, Some(STEP + 1)), None);
    assert_eq!(
        accepted_step(RFC_SECRET, CODE, now, Some(STEP - 1)),
        Some(STEP)
    );
}

#[test]
fn sec_002_only_six_ascii_digits_are_a_code() {
    let now = at(STEP * 30);
    for typed in [
        "",
        "05047",
        "0504710",
        " 050471",
        "050471 ",
        "05047a",
        "05 471",
        "-50471",
        "+50471",
        "０５０４７１",
        "05047\u{0}",
    ] {
        assert_eq!(
            accepted_step(RFC_SECRET, typed, now, None),
            None,
            "{typed:?}"
        );
    }
}

#[test]
fn sec_002_times_at_and_before_the_epoch_do_not_panic() {
    // Step 0 at the epoch; its neighbour step -1 has no code.
    assert_eq!(step_at(at(0)), 0);
    assert_eq!(step_at(at(-1)), -1);
    assert_eq!(accepted_step(RFC_SECRET, "755224", at(0), None), Some(0));
    assert_eq!(accepted_step(RFC_SECRET, "755224", at(-1), None), Some(0));
    assert_eq!(accepted_step(RFC_SECRET, "755224", at(-31), None), None);
    let far = DateTime::<Utc>::MAX_UTC;
    assert_eq!(
        accepted_step(RFC_SECRET, "000000", far, Some(i64::MAX)),
        None
    );
}

#[test]
fn sec_002_r_010_the_vault_context_names_the_operator() {
    let id = "0190a000-0000-7000-8000-000000000001".parse().unwrap();
    assert_eq!(
        context(id),
        "access.operators.totp:0190a000-0000-7000-8000-000000000001"
    );
}

#[test]
fn sec_002_the_enrolment_uri_holds_the_base32_secret_and_parameters() {
    let uri = otpauth_uri(RFC_SECRET, "operator");
    assert_eq!(
        uri.as_str(),
        "otpauth://totp/Anima:operator?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ\
         &issuer=Anima&algorithm=SHA1&digits=6&period=30"
    );
    // RFC 4648 section 10 vectors, without padding.
    for (bytes, encoded) in [
        (&b"f"[..], "MY"),
        (b"fo", "MZXQ"),
        (b"foo", "MZXW6"),
        (b"foob", "MZXW6YQ"),
        (b"fooba", "MZXW6YTB"),
        (b"foobar", "MZXW6YTBOI"),
    ] {
        let uri = otpauth_uri(bytes, "a");
        assert!(
            uri.starts_with(&format!("otpauth://totp/Anima:a?secret={encoded}&")),
            "{bytes:?}"
        );
    }
}
