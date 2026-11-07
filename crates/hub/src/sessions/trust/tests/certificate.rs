//! What the hub keeps of a server certificate (SEC-007): the fingerprint in the stored format,
//! the subject cleaned for display, the expiry, and the fingerprint argument as typed.

use chrono::{DateTime, Utc};
use proptest::prelude::*;
use rcgen::{CertificateParams, DnType, KeyPair, date_time_ymd};
use sha2::{Digest, Sha256};

use super::super::certificate::{
    CertificateFingerprint, MAX_SUBJECT, PresentedCertificate, cleaned_subject,
};

/// SHA-256 of no bytes, in the stored format.
const EMPTY: &str = "e3:b0:c4:42:98:fc:1c:14:9a:fb:f4:c8:99:6f:b9:24:\
                     27:ae:41:e4:64:9b:93:4c:a4:95:99:1b:78:52:b8:55";

#[test]
fn sec_007_the_fingerprint_is_sha256_as_lower_case_hex_pairs_joined_by_colons() {
    assert_eq!(CertificateFingerprint::of(b"").as_str(), EMPTY);
}

#[test]
fn sec_007_a_typed_fingerprint_reads_in_either_case_with_space_around_it() {
    let typed = format!("  {}\n", EMPTY.to_ascii_uppercase());
    assert_eq!(typed.parse(), Ok(CertificateFingerprint::of(b"")));
}

#[test]
fn sec_007_a_fingerprint_of_another_shape_is_refused() {
    let short = &EMPTY[..EMPTY.len() - 3];
    let dashes = EMPTY.replace(':', "-");
    let no_separators = EMPTY.replace(':', "");
    let not_hex = EMPTY.replacen("e3", "g3", 1);
    let openssh = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";
    let extra = format!("{EMPTY}:00");
    for text in [
        short,
        &dashes,
        &no_separators,
        &not_hex,
        openssh,
        &extra,
        "",
    ] {
        assert!(text.parse::<CertificateFingerprint>().is_err(), "{text}");
    }
}

#[test]
fn sec_007_a_certificate_gives_its_fingerprint_subject_and_expiry() {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec!["rdp.lan".to_owned()]).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, "DESKTOP-01");
    params.not_after = date_time_ymd(2030, 6, 1);
    let cert = params.self_signed(&key).unwrap();
    let der = cert.der().to_vec();
    let presented = PresentedCertificate::from_der(&der);
    let sha256: Vec<String> = Sha256::digest(&der)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(presented.fingerprint.as_str(), sha256.join(":"));
    assert_eq!(presented.subject.as_deref(), Some("CN=DESKTOP-01"));
    let not_after: DateTime<Utc> = "2030-06-01T00:00:00Z".parse().unwrap();
    assert_eq!(presented.not_after, Some(not_after));
}

#[test]
fn sec_007_bytes_that_are_not_x509_still_have_a_fingerprint() {
    let presented = PresentedCertificate::from_der(b"not a certificate");
    assert_eq!(
        presented.fingerprint,
        CertificateFingerprint::of(b"not a certificate")
    );
    assert_eq!((presented.subject, presented.not_after), (None, None));
}

#[test]
fn sec_007_a_subject_loses_control_and_bidirectional_characters() {
    assert_eq!(
        cleaned_subject("CN=pc\r\n\u{7}\u{85}-01\u{202e}txt.exe"),
        Some("CN=pc-01txt.exe".to_owned())
    );
    assert_eq!(cleaned_subject(" \u{0}\t "), None);
}

#[test]
fn sec_007_a_long_subject_keeps_its_first_1024_characters() {
    let long = "é".repeat(MAX_SUBJECT + 10);
    let kept = cleaned_subject(&long).unwrap();
    assert_eq!(kept.chars().count(), MAX_SUBJECT);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    /// What the database's `server_certificates_subject_check` requires.
    #[test]
    fn sec_007_a_cleaned_subject_always_fits_the_table(text in any::<String>()) {
        if let Some(kept) = cleaned_subject(&text) {
            prop_assert!(!kept.is_empty() && kept.chars().count() <= MAX_SUBJECT);
            prop_assert!(!kept.chars().any(char::is_control));
        }
    }

    #[test]
    fn sec_007_no_bytes_panic_the_certificate_reader(
        der in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        let presented = PresentedCertificate::from_der(&der);
        prop_assert_eq!(presented.fingerprint, CertificateFingerprint::of(&der));
    }
}
