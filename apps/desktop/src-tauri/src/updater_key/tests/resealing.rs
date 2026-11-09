//! An imported key sealed with a weak scrypt cost is sealed anew, a new password has at least 12
//! characters, and an unlocked key must match the public line stored beside it (D-19, M-1).

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign::KeyPair;

use super::tauri_key_file;
use crate::updater_key::{
    UpdaterKeyError,
    sealed::{Imported, imported, scrypt_cost, unlocked, unseal},
    secret::{MEM_AT, OPS_AT, SALT_AT, SEALED_AT},
};

const SALT: [u8; 32] = [7; 32];
const OLD_PASSWORD: &str = "short pw";
const NEW_PASSWORD: &str = "a new password";

/// A minisign key file sealed with `password` at scrypt N = 2^10, below minisign's 2^15.
fn weak_key_file(password: &str) -> String {
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let plain = pair.sk.to_box(None).unwrap().into_string();
    let mut bytes = STANDARD.decode(plain.lines().nth(1).unwrap()).unwrap();
    bytes[2..4].copy_from_slice(b"Sc");
    bytes[SALT_AT..OPS_AT].copy_from_slice(&[9; 32]);
    bytes[OPS_AT..MEM_AT].copy_from_slice(&32_768_u64.to_le_bytes());
    bytes[MEM_AT..SEALED_AT].copy_from_slice(&33_554_432_u64.to_le_bytes());
    unseal(&mut bytes, password.as_bytes()).unwrap();
    format!("untrusted comment: weak\n{}\n", STANDARD.encode(&bytes))
}

/// scrypt `log2(N)` of stored key bytes, and their salt.
fn cost_and_salt(stored: &str) -> (u8, Vec<u8>) {
    let bytes = STANDARD.decode(stored).unwrap();
    let ops = u64::from_le_bytes(bytes[OPS_AT..MEM_AT].try_into().unwrap());
    let mem = u64::from_le_bytes(bytes[MEM_AT..SEALED_AT].try_into().unwrap());
    (
        scrypt_cost(ops, mem).unwrap().0,
        bytes[SALT_AT..OPS_AT].to_vec(),
    )
}

#[test]
fn d_19_a_weakly_sealed_key_is_sealed_anew_with_the_same_password_or_a_new_one() {
    let file = weak_key_file(OLD_PASSWORD);
    assert_eq!(
        cost_and_salt(file.lines().nth(1).unwrap()),
        (10, vec![9; 32])
    );
    assert_eq!(
        imported(file.as_bytes(), Some("wrong"), None, &SALT).err(),
        Some(UpdaterKeyError::WrongPassword)
    );
    let Imported { key, stored } =
        imported(file.as_bytes(), Some(OLD_PASSWORD), None, &SALT).unwrap();
    let public = key.public_key().line();
    assert_eq!(cost_and_salt(&stored), (15, SALT.to_vec()));
    assert!(unlocked(&stored, OLD_PASSWORD, &public).is_ok());
    let Imported { stored, .. } = imported(
        file.as_bytes(),
        Some(OLD_PASSWORD),
        Some(NEW_PASSWORD),
        &SALT,
    )
    .unwrap();
    assert_eq!(cost_and_salt(&stored), (15, SALT.to_vec()));
    assert!(unlocked(&stored, NEW_PASSWORD, &public).is_ok());
    assert_eq!(
        unlocked(&stored, OLD_PASSWORD, &public).err(),
        Some(UpdaterKeyError::WrongPassword)
    );
    assert_eq!(
        imported(
            file.as_bytes(),
            Some(OLD_PASSWORD),
            Some("too short"),
            &SALT
        )
        .err(),
        Some(UpdaterKeyError::PasswordTooShort)
    );
}

#[test]
fn d_19_a_key_sealed_at_minisigns_cost_is_kept_as_it_is() {
    let (file, _) = tauri_key_file(Some(NEW_PASSWORD));
    let Imported { stored, .. } =
        imported(file.as_bytes(), Some(NEW_PASSWORD), None, &SALT).unwrap();
    let (n_log2, salt) = cost_and_salt(&stored);
    assert!(n_log2 >= 15);
    assert_ne!(salt, SALT.to_vec(), "a strong seal was sealed anew");
}

#[test]
fn d_19_a_new_password_needs_twelve_characters() {
    let (file, _) = tauri_key_file(None);
    for short in ["a", "eleven char", "ëëëëëëëëëëë"] {
        assert_eq!(
            imported(file.as_bytes(), None, Some(short), &SALT).err(),
            Some(UpdaterKeyError::PasswordTooShort),
            "{short:?}"
        );
    }
    assert!(imported(file.as_bytes(), None, Some("twelve chars"), &SALT).is_ok());
}

#[test]
fn d_19_a_stored_key_that_does_not_match_its_public_line_is_refused() {
    let (file, _) = tauri_key_file(None);
    let Imported { key, stored } =
        imported(file.as_bytes(), None, Some(NEW_PASSWORD), &SALT).unwrap();
    let (other, _) = tauri_key_file(None);
    let other = imported(other.as_bytes(), None, Some(NEW_PASSWORD), &SALT)
        .unwrap()
        .key
        .public_key()
        .line();
    assert_eq!(
        unlocked(&stored, NEW_PASSWORD, &other).err(),
        Some(UpdaterKeyError::Mismatch)
    );
    assert!(unlocked(&stored, NEW_PASSWORD, &key.public_key().line()).is_ok());
}
