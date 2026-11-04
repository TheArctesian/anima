//! The listening edge: the bind table (NET-003, SEC-008), certificate loading without key
//! bytes in errors (SEC-001, LOG-007), the reload (R-021 H-2) and TLS serving. Certificates are
//! throwaway ones made here with `rcgen`; no real key file is read (SEC-004).

mod bind;
mod edge;
mod health;
mod reload;
mod serving;

use std::{
    fs::File,
    io::Write as _,
    sync::Arc,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, TimeZone, Utc};
use tokio_rustls::rustls::crypto::{CryptoProvider, ring};

use crate::clock::FixedClock;

use super::{
    Edge, TlsFiles,
    certificate::{self, Fault, Pem},
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

/// An edge that permits no network, so only this host's own connections pass (no database).
fn nowhere() -> Edge {
    Edge::permitting(&[], Arc::new(FixedClock(now())))
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(ring::default_provider())
}

/// A self-signed certificate for `localhost` and its private key, both as PEM.
struct Throwaway {
    cert: String,
    key: String,
    der: Vec<u8>,
}

fn throwaway() -> Throwaway {
    let made = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    Throwaway {
        cert: made.cert.pem(),
        key: made.signing_key.serialize_pem(),
        der: made.cert.der().to_vec(),
    }
}

/// Two files in a temporary folder that lives as long as the value.
struct Files {
    _dir: tempfile::TempDir,
    paths: TlsFiles,
}

impl Files {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let paths = TlsFiles {
            cert: dir.path().join("cert.pem"),
            key: dir.path().join("key.pem"),
        };
        Self { _dir: dir, paths }
    }

    /// One `tls.pem` named by both variables, as the deployment has it (WP-04).
    fn single() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tls.pem");
        let paths = TlsFiles {
            cert: path.clone(),
            key: path,
        };
        Self { _dir: dir, paths }
    }

    /// Writes both files and sets their modification time `seconds` after a fixed instant,
    /// so a rewrite is always seen as a change.
    fn write(&self, cert: &str, key: &str, seconds: u64) {
        let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1_900_000_000 + seconds);
        for (path, text) in [(&self.paths.cert, cert), (&self.paths.key, key)] {
            let mut file = File::create(path).unwrap();
            file.write_all(text.as_bytes()).unwrap();
            file.set_modified(modified).unwrap();
        }
    }
}

fn pem(cert: &str, key: &str) -> Pem {
    Pem {
        cert: cert.as_bytes().to_vec(),
        key: zeroize::Zeroizing::new(key.as_bytes().to_vec()),
    }
}

/// The served certificate of a key loaded from `cert` and `key`.
fn load(cert: &str, key: &str) -> Result<Vec<u8>, Fault> {
    let key = certificate::certified_key(&pem(cert, key), &provider())?;
    Ok(key.cert[0].to_vec())
}

#[test]
fn sec_001_a_throwaway_certificate_and_its_key_load() {
    let made = throwaway();
    assert_eq!(load(&made.cert, &made.key), Ok(made.der));
}

#[test]
fn h_2_one_file_with_the_key_and_a_chain_loads_its_first_certificate() {
    let (leaf, issuer) = (throwaway(), throwaway());
    let tls_pem = format!("{}{}{}", leaf.key, leaf.cert, issuer.cert);
    let key = certificate::certified_key(&pem(&tls_pem, &tls_pem), &provider()).unwrap();
    assert_eq!(key.cert.len(), 2);
    assert_eq!(key.cert[0].to_vec(), leaf.der);
    let wrong_order = format!("{}{}{}", leaf.key, issuer.cert, leaf.cert);
    assert_eq!(
        load(&wrong_order, &wrong_order),
        Err(Fault::KeyMismatch),
        "the key must match the first certificate"
    );
}

#[test]
fn sec_001_a_key_of_another_certificate_is_refused() {
    let (one, other) = (throwaway(), throwaway());
    assert_eq!(load(&one.cert, &other.key), Err(Fault::KeyMismatch));
}

#[test]
fn sec_001_files_without_a_certificate_or_key_are_refused_by_kind() {
    let made = throwaway();
    assert_eq!(load("", &made.key), Err(Fault::NoCertificate));
    assert_eq!(load(&made.key, &made.key), Err(Fault::NoCertificate));
    assert_eq!(load(&made.cert, ""), Err(Fault::KeyPem));
    assert_eq!(load(&made.cert, &made.cert), Err(Fault::KeyPem));
    let broken = "-----BEGIN CERTIFICATE-----\n!!!!\n-----END CERTIFICATE-----\n";
    assert_eq!(load(broken, &made.key), Err(Fault::CertificatePem));
    let not_der = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n";
    assert_eq!(load(not_der, &made.key), Err(Fault::CertificateInvalid));
}

/// Key-looking text that would leak if an error quoted the key file.
const SECRET: &str = "c2VjcmV0LWtleS1ieXRlcw";

#[test]
fn log_007_errors_for_a_bad_key_hold_no_key_bytes() {
    let made = throwaway();
    let keys = [
        format!("-----BEGIN PRIVATE KEY-----\n{SECRET}!\n-----END PRIVATE KEY-----\n"),
        format!("-----BEGIN PRIVATE KEY-----\n{SECRET}\n"),
        format!("{SECRET}-----BEGIN PRIVATE KEY-----\n"),
        format!("-----BEGIN PRIVATE KEY-----\n{SECRET}==\n-----END PRIVATE KEY-----\n"),
    ];
    for key in keys {
        let fault = load(&made.cert, &key).unwrap_err();
        let shown = format!("{fault} {fault:?} {}", super::Error::Certificate(fault));
        assert!(!shown.contains(SECRET), "{shown}");
        assert!(!shown.contains("PRIVATE"), "{shown}");
        assert!(
            matches!(fault, Fault::KeyPem | Fault::KeyUnsupported),
            "{fault:?}"
        );
    }
}

#[test]
fn sec_001_unreadable_and_oversized_files_are_refused_by_kind() {
    let files = Files::new();
    let missing = certificate::read(&files.paths).err();
    assert!(
        matches!(
            missing,
            Some(Fault::CertificateFile(std::io::ErrorKind::NotFound))
        ),
        "{missing:?}"
    );
    let made = throwaway();
    files.write(&made.cert, &"k".repeat(1024 * 1024 + 1), 0);
    assert_eq!(
        certificate::read(&files.paths).err(),
        Some(Fault::KeyTooLarge)
    );
    files.write(&"c".repeat(1024 * 1024 + 1), &made.key, 0);
    assert_eq!(
        certificate::read(&files.paths).err(),
        Some(Fault::CertificateTooLarge)
    );
    files.write(&"c".repeat(1024 * 1024), &made.key, 0);
    assert!(certificate::read(&files.paths).is_ok(), "1 MiB is allowed");
    std::fs::remove_file(&files.paths.key).unwrap();
    assert!(matches!(
        certificate::read(&files.paths).err(),
        Some(Fault::KeyFile(std::io::ErrorKind::NotFound))
    ));
}
