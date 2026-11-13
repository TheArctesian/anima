//! Tests of the pure publishing rules: `SigV4` against the AWS examples, the feed check, the file
//! set, the hub's updater key and its signature, and property tests of the parsers (BLD-060 to
//! BLD-069, D-24, R-108, R-117); and the row lock a try that leaves its job gives up (D-25).

mod feeds;
mod files;
mod holds;
mod parsers;
mod signatures;
mod sigv4;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign::KeyPair;

use super::updater_key::UpdaterKey;
use crate::builds::model::UploadedFile;

fn file(name: &str, bytes: &[u8]) -> UploadedFile {
    UploadedFile {
        name: name.to_owned(),
        sha256: super::sigv4::sha256_hex(bytes),
        size: i64::try_from(bytes.len()).unwrap(),
    }
}

/// A throwaway key made by the reference minisign, as `tauri signer generate` writes it: the key
/// file (base64 of the minisign text), the `plugins.updater.pubkey` text, and the pair.
fn tauri_key(password: Option<&str>) -> (String, String, KeyPair) {
    let pair = match password {
        Some(password) => KeyPair::generate_encrypted_keypair(Some(password.to_owned())).unwrap(),
        None => KeyPair::generate_unencrypted_keypair().unwrap(),
    };
    let file = STANDARD.encode(pair.sk.to_box(None).unwrap().into_string());
    let public = STANDARD.encode(pair.pk.to_box().unwrap().into_string());
    (file, public, pair)
}

/// An opened throwaway updater key.
fn updater_key() -> UpdaterKey {
    let (file, public, _) = tauri_key(None);
    UpdaterKey::from_parts(file.as_bytes(), b"", &public).unwrap()
}
