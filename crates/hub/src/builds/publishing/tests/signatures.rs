//! The updater signature, checked against the reference minisign (BLD-064, UPD-002, R-108).

use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign::KeyPair;

use crate::builds::publishing::signature::{UpdaterKey, holds, signature_of};

const DATA: &[u8] = b"an AppImage, as far as signing cares";

/// The `plugins.updater.pubkey` text of a fresh key pair, and the pair.
fn key_pair() -> (String, KeyPair) {
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let file = pair.pk.to_box().unwrap().into_string();
    (STANDARD.encode(file), pair)
}

/// The feed `signature` text of `data` signed by `pair` for `version`.
fn signed(pair: &KeyPair, data: &[u8], version: &str) -> String {
    let comment = format!("timestamp:1700000000\tfile:Anima.AppImage\tversion:{version}");
    let signature = minisign::sign(
        Some(&pair.pk),
        &pair.sk,
        Cursor::new(data),
        Some(&comment),
        None,
    )
    .unwrap();
    STANDARD.encode(signature.into_string())
}

fn verifies(key: &str, signature: &str, data: &[u8], version: &str) -> bool {
    let key = UpdaterKey::parse(key).unwrap();
    let signature = signature_of(signature).unwrap();
    let Some(mut verifier) = key.verifier(&signature) else {
        return false;
    };
    verifier.update(data);
    holds(verifier, &signature, version)
}

#[test]
fn bld_064_a_signature_of_the_file_for_the_version_holds() {
    let (key, pair) = key_pair();
    assert!(verifies(
        &key,
        &signed(&pair, DATA, "0.1.12"),
        DATA,
        "0.1.12"
    ));
}

#[test]
fn bld_064_another_file_version_or_key_does_not_hold() {
    let (key, pair) = key_pair();
    let signature = signed(&pair, DATA, "0.1.12");
    assert!(!verifies(
        &key,
        &signature,
        b"a tampered AppImage",
        "0.1.12"
    ));
    assert!(!verifies(&key, &signature, DATA, "0.1.13"));
    assert!(!verifies(
        &key,
        &signed(&pair, DATA, "0.1.12\tversion:0.1.13"),
        DATA,
        "0.1.12"
    ));
    let (other_key, _) = key_pair();
    assert!(!verifies(&other_key, &signature, DATA, "0.1.12"));
}

#[test]
fn r_108_text_that_is_not_a_key_or_signature_is_refused() {
    assert!(UpdaterKey::parse("REPLACE_WITH_OPERATOR_PUBLIC_KEY").is_none());
    assert!(UpdaterKey::parse(&STANDARD.encode("untrusted comment: x\nnot a key\n")).is_none());
    assert!(signature_of("not base64 !").is_none());
}

/// The placeholder `tauri.conf.json` carries until the operator's step (WP-04b D-3).
const PLACEHOLDER: &str = "REPLACE_WITH_OPERATOR_PUBLIC_KEY";

/// R-108: `ANIMA_UPDATER_PUBLIC_KEY` must be the app's `plugins.updater.pubkey`, so the app's
/// value must stay a key the hub reads. Until the operator commits the key, the placeholder is
/// the only other value allowed.
#[test]
fn r_108_the_app_updater_key_is_one_the_hub_reads() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src-tauri/tauri.conf.json"
    );
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let pubkey = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
    assert!(
        pubkey == PLACEHOLDER || UpdaterKey::parse(pubkey).is_some(),
        "plugins.updater.pubkey is neither the placeholder nor a minisign public key"
    );
}
