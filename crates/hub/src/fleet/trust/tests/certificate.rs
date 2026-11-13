//! What the hub keeps of a server certificate (SEC-007): the fingerprint in the stored format,
//! the subject cleaned for display, and the expiry. The fingerprint's forms are tested in the
//! `fingerprint` leaf.

use chrono::{DateTime, Utc};
use proptest::prelude::*;
use rcgen::{CertificateParams, DnType, KeyPair, date_time_ymd};
use sha2::{Digest, Sha256};

use super::super::certificate::{MAX_SUBJECT, PresentedCertificate, cleaned_subject};
use crate::fingerprint::CertificateFingerprint;

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
    assert_eq!(presented.fingerprint.colon_pairs(), sha256.join(":"));
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
    assert_eq!(
        cleaned_subject("CN=a\u{061c}b\u{2028}c\u{200b}d\u{feff}").as_deref(),
        Some("CN=abcd"),
        "format characters and separators go too"
    );
    assert_eq!(
        cleaned_subject("CN=a\u{2065}b\u{e0002}c").as_deref(),
        Some("CN=abc"),
        "the widest table covers unassigned format slots"
    );
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
