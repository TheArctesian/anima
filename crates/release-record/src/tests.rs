//! Each key here is made in the test by the reference minisign crate; no real key is read
//! (UPD-012).

mod checks;
mod fields;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest as _, Sha256};
use updater_signature::{Prehash, PublicKey, SecretKey, Signed};

use crate::{Database, Expected, FORMAT, File, Program, Record, Target, Version, sign};

const DAEMON: &[u8] = b"the bytes of anima-daemon, as far as a check cares";
const TIME: u64 = 1_800_000_000;

fn key() -> SecretKey {
    let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let text = STANDARD.encode(pair.sk.to_box(None).unwrap().into_string());
    SecretKey::from_key_file(text.as_bytes(), None).unwrap()
}

fn version(text: &str) -> Version {
    text.parse().unwrap()
}

fn hex(bytes: &[u8]) -> String {
    crate::check::to_hex(&Sha256::digest(bytes))
}

fn daemon_file() -> File {
    File {
        name: "anima-daemon".to_owned(),
        program: Program::Daemon,
        size: DAEMON.len() as u64,
        sha256: hex(DAEMON),
    }
}

/// The record of 0.3.0 for Linux, which replaces each version from 0.2.0.
fn record() -> Record {
    Record {
        format: FORMAT,
        version: version("0.3.0"),
        commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        target: Target::LinuxX8664,
        replaces_from: version("0.2.0"),
        database: Database::Compatible,
        files: vec![daemon_file()],
    }
}

fn expected() -> Expected {
    Expected {
        version: version("0.3.0"),
        target: Target::LinuxX8664,
        program: Program::Daemon,
    }
}

/// A signature of `bytes` with any trusted comment, also one that [`sign`] never writes.
fn raw_signature(key: &SecretKey, bytes: &[u8], file_name: &str, version: &str) -> String {
    let mut hash = Prehash::new();
    hash.update(bytes);
    key.sign(
        &hash.finish(),
        &Signed {
            timestamp: TIME,
            file_name,
            version,
        },
    )
    .unwrap()
}

fn signed(key: &SecretKey) -> (Vec<u8>, String, PublicKey) {
    let (bytes, signature) = sign(&record(), key, TIME).unwrap();
    (bytes, signature, key.public_key())
}
