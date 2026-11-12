//! The hub's updater key and its signature (R-116, BLD-045, BLD-046, UPD-002): the key opens
//! only with its password and only as the key of `ANIMA_UPDATER_PUBLIC_KEY`, its signature is
//! accepted by the reference minisign for the version, and no key text reaches an error. Every
//! key is made here; no real key file is read.

use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign::SignatureBox;
use updater_signature::{Prehash, PublicKey, SignatureError, Signed};

use super::{tauri_key, updater_key};
use crate::{
    builds::publishing::updater_key::{UpdaterKey, UpdaterKeyError},
    config::Releases,
};

const PASSWORD: &str = "correct horse battery";
const DATA: &[u8] = b"an AppImage, as far as signing cares";

fn digest(data: &[u8]) -> [u8; 64] {
    let mut hash = Prehash::new();
    hash.update(data);
    hash.finish()
}

fn signed(version: &str) -> Signed<'_> {
    Signed {
        timestamp: 1_888_000_000,
        file_name: "Anima_0.1.12_amd64.AppImage",
        version,
    }
}

#[test]
fn r_116_an_encrypted_key_opens_with_its_password_file() {
    let (file, public, _) = tauri_key(Some(PASSWORD));
    for password in [
        PASSWORD.to_owned(),
        format!("{PASSWORD}\n"),
        format!("{PASSWORD}\r\n"),
    ] {
        assert!(UpdaterKey::from_parts(file.as_bytes(), password.as_bytes(), &public).is_ok());
    }
    let (open, public, _) = tauri_key(None);
    assert!(UpdaterKey::from_parts(open.as_bytes(), b"", &public).is_ok());
}

#[test]
fn r_116_a_wrong_or_missing_password_turns_publishing_off_with_a_fixed_error() {
    let (file, public, _) = tauri_key(Some(PASSWORD));
    let open = |password: &[u8]| UpdaterKey::from_parts(file.as_bytes(), password, &public).err();
    assert_eq!(
        open(b"wrong password"),
        Some(UpdaterKeyError::Open(SignatureError::WrongPassword))
    );
    assert_eq!(
        open(format!("{PASSWORD} ").as_bytes()),
        Some(UpdaterKeyError::Open(SignatureError::WrongPassword))
    );
    assert_eq!(
        open(b""),
        Some(UpdaterKeyError::Open(SignatureError::PasswordNeeded))
    );
    assert_eq!(
        open(&[0xff, 0xfe]),
        Some(UpdaterKeyError::PasswordFile(
            std::io::ErrorKind::InvalidData
        ))
    );
}

#[test]
fn r_116_a_key_that_is_not_the_configured_public_key_is_refused() {
    let (file, _, _) = tauri_key(None);
    let (_, other_public, _) = tauri_key(None);
    assert_eq!(
        UpdaterKey::from_parts(file.as_bytes(), b"", &other_public).err(),
        Some(UpdaterKeyError::Mismatch)
    );
    for public in [
        "",
        "REPLACE_WITH_OPERATOR_PUBLIC_KEY",
        &STANDARD.encode("not a key"),
    ] {
        assert_eq!(
            UpdaterKey::from_parts(file.as_bytes(), b"", public).err(),
            Some(UpdaterKeyError::PublicKey)
        );
    }
    assert_eq!(
        UpdaterKey::from_parts(b"not a key file", b"", &tauri_key(None).1).err(),
        Some(UpdaterKeyError::Open(SignatureError::Format))
    );
}

fn releases(folder: &std::path::Path, public: Option<String>) -> Releases {
    Releases {
        endpoint: "http://127.0.0.1:9".to_owned(),
        bucket: "anima-releases".to_owned(),
        public_base: "http://127.0.0.1:9".to_owned(),
        access_key_id_file: folder.join("id"),
        secret_access_key_file: folder.join("secret"),
        updater_public_key: public,
        updater_key_file: folder.join("updater_key"),
        updater_key_password_file: folder.join("updater_key_password"),
    }
}

#[test]
fn r_116_sec_004_the_key_and_its_password_are_read_from_their_files() {
    let folder = tempfile::tempdir().unwrap();
    let (file, public, _) = tauri_key(Some(PASSWORD));
    let releases = releases(folder.path(), Some(public.clone()));
    assert_eq!(
        UpdaterKey::open(&releases).err(),
        Some(UpdaterKeyError::KeyFile(std::io::ErrorKind::NotFound))
    );
    std::fs::write(&releases.updater_key_file, &file).unwrap();
    assert_eq!(
        UpdaterKey::open(&releases).err(),
        Some(UpdaterKeyError::PasswordFile(std::io::ErrorKind::NotFound))
    );
    std::fs::write(&releases.updater_key_password_file, format!("{PASSWORD}\n")).unwrap();
    assert!(UpdaterKey::open(&releases).is_ok());
    let unset = self::releases(folder.path(), None);
    assert_eq!(
        UpdaterKey::open(&unset).err(),
        Some(UpdaterKeyError::NoPublicKey)
    );
    std::fs::write(&releases.updater_key_file, "A".repeat(4097)).unwrap();
    assert_eq!(
        UpdaterKey::open(&releases).err(),
        Some(UpdaterKeyError::KeyFile(std::io::ErrorKind::InvalidData))
    );
}

#[test]
fn bld_046_the_hub_signature_holds_for_the_reference_minisign_and_names_the_version() {
    let (file, public, pair) = tauri_key(Some(PASSWORD));
    let key = UpdaterKey::from_parts(file.as_bytes(), PASSWORD.as_bytes(), &public).unwrap();
    let text = key.sign(&digest(DATA), &signed("0.1.12")).unwrap();
    let decoded = String::from_utf8(STANDARD.decode(&text).unwrap()).unwrap();
    let parsed = SignatureBox::from_string(&decoded).unwrap();
    minisign::verify(&pair.pk, &parsed, Cursor::new(DATA), true, false, false).unwrap();
    assert!(
        parsed
            .trusted_comment()
            .unwrap()
            .ends_with("\tversion:0.1.12")
    );
    // The same digest and comment sign the same text: a retry writes the same `.sig` (D-25).
    assert_eq!(key.sign(&digest(DATA), &signed("0.1.12")).unwrap(), text);
}

#[test]
fn bld_046_a_signature_holds_only_over_the_bytes_whose_digest_was_signed() {
    let key = updater_key();
    let text = key.sign(&digest(DATA), &signed("0.1.12")).unwrap();
    let check = key.check(&text).unwrap();
    let mut other = check.stream().unwrap();
    other.update(b"bytes other than the verified ones");
    assert_eq!(other.finish("0.1.12"), Err(SignatureError::Signature));
    let mut same = check.stream().unwrap();
    same.update(&DATA[..7]);
    same.update(&DATA[7..]);
    assert_eq!(same.finish("0.1.12"), Ok(()));
    let mut version = check.stream().unwrap();
    version.update(DATA);
    assert_eq!(version.finish("0.1.13"), Err(SignatureError::SignedVersion));
}

#[test]
fn log_007_the_key_has_no_debug_and_no_error_carries_key_text() {
    static_assertions::assert_not_impl_any!(UpdaterKey: std::fmt::Debug, std::fmt::Display, Clone);
    let (file, public, _) = tauri_key(Some(PASSWORD));
    let failures = [
        UpdaterKey::from_parts(file.as_bytes(), b"wrong password", &public).err(),
        UpdaterKey::from_parts(file.as_bytes(), PASSWORD.as_bytes(), &tauri_key(None).1).err(),
        UpdaterKey::from_parts(file.as_bytes(), PASSWORD.as_bytes(), &file).err(),
        UpdaterKey::from_parts(public.as_bytes(), PASSWORD.as_bytes(), &public).err(),
    ];
    let key_line = String::from_utf8(STANDARD.decode(&file).unwrap()).unwrap();
    let key_line = key_line.lines().nth(1).unwrap().to_owned();
    for error in failures {
        let error = error.unwrap();
        let shown = format!("{error} {error:?}");
        for secret in [file.as_str(), key_line.as_str(), PASSWORD, "wrong password"] {
            assert!(!shown.contains(secret), "{shown}");
        }
    }
}

/// R-108, R-116: `ANIMA_UPDATER_PUBLIC_KEY` must be the app's `plugins.updater.pubkey`, so the
/// app's value must be a key the hub reads: the operator's, never the old placeholder.
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
        PublicKey::from_key_file(pubkey).is_ok(),
        "plugins.updater.pubkey is not a minisign public key"
    );
}
