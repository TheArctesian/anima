//! The key store keeps the updater key sealed with a password, never as a bare seed (D-19), and
//! the opened key lives only in this process's memory until it is locked. Every key is made in
//! the test.

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign::KeyPair;

use super::{DATA, PASSWORD, signed, tauri_key_file};
use crate::updater_key::{
    UnlockedKey, UpdaterKeyError,
    sealed::{Imported, imported, unlocked},
    secret::SecretKey,
    signature::{prehash, verify},
};

const SALT: [u8; 32] = [7; 32];
const NEW_PASSWORD: &str = "a new password";

/// The key's bare seed: what must never be in the store.
fn seed_of(key: &SecretKey) -> Vec<u8> {
    key.sealed_body()[8..40].to_vec()
}

fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Checks a stored text is sealed minisign key bytes without the bare seed.
fn assert_sealed(stored: &str, key: &SecretKey) {
    let bytes = STANDARD.decode(stored).unwrap();
    assert_eq!(
        &bytes[..6],
        b"EdScB2",
        "the store holds a sealed minisign key"
    );
    assert!(
        !holds(&bytes, &seed_of(key)),
        "the store holds the bare seed"
    );
    assert!(!holds(
        stored.as_bytes(),
        STANDARD.encode(seed_of(key)).as_bytes()
    ));
}

#[test]
fn bld_045_d_19_a_key_file_with_a_password_is_stored_as_it_is_once_the_password_opens_it() {
    let (file, public) = tauri_key_file(Some(PASSWORD));
    let Imported { key, stored } = imported(file.as_bytes(), Some(PASSWORD), None, &SALT).unwrap();
    assert_eq!(key.public_key().line(), public.to_base64());
    assert_sealed(&stored, &key);
    let opened = unlocked(&stored, PASSWORD, &key.public_key().line()).unwrap();
    assert_eq!(opened.public_key(), key.public_key());
    assert_eq!(
        imported(file.as_bytes(), Some("wrong"), None, &SALT).err(),
        Some(UpdaterKeyError::WrongPassword)
    );
    assert_eq!(
        imported(file.as_bytes(), None, Some(NEW_PASSWORD), &SALT).err(),
        Some(UpdaterKeyError::PasswordNeeded)
    );
}

#[test]
fn bld_045_d_19_a_key_file_without_a_password_needs_a_new_one_and_is_sealed_with_it() {
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let plain = pair.sk.to_box(None).unwrap().into_string();
    let (empty, _) = tauri_key_file(Some(""));
    for file in [plain.as_str(), empty.as_str()] {
        for no_password in [None, Some("")] {
            assert_eq!(
                imported(file.as_bytes(), None, no_password, &SALT).err(),
                Some(UpdaterKeyError::NewPasswordNeeded)
            );
        }
        let Imported { key, stored } =
            imported(file.as_bytes(), None, Some(NEW_PASSWORD), &SALT).unwrap();
        assert_sealed(&stored, &key);
        assert_eq!(
            unlocked(&stored, NEW_PASSWORD, &key.public_key().line())
                .unwrap()
                .public_key(),
            key.public_key()
        );
    }
    // The reference minisign opens what was sealed here, with the new password.
    let Imported { stored, .. } =
        imported(plain.as_bytes(), None, Some(NEW_PASSWORD), &SALT).unwrap();
    let text = format!("untrusted comment: sealed by anima\n{}\n", stored.as_str());
    let reference = minisign::SecretKeyBox::from_string(&text)
        .unwrap()
        .into_secret_key(Some(NEW_PASSWORD.to_owned()));
    assert!(reference.is_ok());
}

#[test]
fn bld_045_d_19_a_wrong_unlock_password_is_refused_and_a_bare_stored_key_is_never_used() {
    let (file, _) = tauri_key_file(None);
    let Imported { key, stored } =
        imported(file.as_bytes(), None, Some(NEW_PASSWORD), &SALT).unwrap();
    assert_eq!(
        unlocked(&stored, "wrong", &key.public_key().line()).err(),
        Some(UpdaterKeyError::WrongPassword)
    );
    assert_eq!(
        unlocked(&stored, "", &key.public_key().line()).err(),
        Some(UpdaterKeyError::WrongPassword)
    );
    // The bare form T2 first stored (key number, seed, public key) is not a key here.
    let bare = STANDARD.encode(key.sealed_body().as_slice());
    assert_eq!(
        unlocked(&bare, NEW_PASSWORD, &key.public_key().line()).err(),
        Some(UpdaterKeyError::Format)
    );
    // Nor is an unencrypted minisign key, whose seed would be in the clear.
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let plain = pair.sk.to_box(None).unwrap().into_string();
    let unencrypted = plain.lines().nth(1).unwrap();
    assert_eq!(
        unlocked(unencrypted, NEW_PASSWORD, &key.public_key().line()).err(),
        Some(UpdaterKeyError::Format)
    );
}

#[test]
fn bld_045_d_19_signing_fails_while_locked_and_locking_drops_the_key() {
    let (file, _) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    let public = key.public_key();
    let digest = prehash(DATA).unwrap();
    let held = UnlockedKey::default();
    assert!(!held.is_unlocked());
    assert_eq!(
        held.sign(&digest, &signed("0.1.12")).err(),
        Some(UpdaterKeyError::Locked)
    );
    held.set(key);
    let signature = held.sign(&digest, &signed("0.1.12")).unwrap();
    assert_eq!(verify(&public, DATA, &signature, "0.1.12"), Ok(()));
    held.lock();
    assert!(!held.is_unlocked());
    assert_eq!(
        held.sign(&digest, &signed("0.1.12")).err(),
        Some(UpdaterKeyError::Locked)
    );
}
