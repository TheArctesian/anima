//! Key files as `tauri signer generate` and minisign write them: opened with their password,
//! refused when damaged, and never made to cost more scrypt work than N = 2^20, p = 1 (BLD-045).

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign::KeyPair;

use super::{PASSWORD, tauri_key_file, tauri_public_key};
use crate::{
    PublicKey, SecretKey, SignatureError,
    key_file::{MEM_AT, OPS_AT, SEALED_AT},
};

#[test]
fn bld_045_an_encrypted_tauri_key_file_opens_with_its_password() {
    let (file, public) = tauri_key_file(Some(PASSWORD));
    let key = SecretKey::from_key_file(file.as_bytes(), Some(PASSWORD)).unwrap();
    assert_eq!(key.public_key().line(), public.to_base64());
}

#[test]
fn bld_045_a_key_file_encrypted_with_no_password_opens_without_one() {
    let (file, public) = tauri_key_file(Some(""));
    for no_password in [None, Some("")] {
        let key = SecretKey::from_key_file(file.as_bytes(), no_password).unwrap();
        assert_eq!(key.public_key().line(), public.to_base64());
    }
}

#[test]
fn bld_045_the_plain_minisign_text_and_an_unencrypted_key_open_too() {
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let text = pair.sk.to_box(None).unwrap().into_string();
    let key = SecretKey::from_key_file(text.as_bytes(), None).unwrap();
    assert_eq!(key.public_key().line(), pair.pk.to_base64());
}

#[test]
fn bld_045_a_wrong_or_missing_password_is_named() {
    let (file, _) = tauri_key_file(Some(PASSWORD));
    assert_eq!(
        SecretKey::from_key_file(file.as_bytes(), Some("wrong")).err(),
        Some(SignatureError::WrongPassword)
    );
    assert_eq!(
        SecretKey::from_key_file(file.as_bytes(), Some("")).err(),
        Some(SignatureError::WrongPassword)
    );
    assert_eq!(
        SecretKey::from_key_file(file.as_bytes(), None).err(),
        Some(SignatureError::PasswordNeeded)
    );
    let long = "x".repeat(1025);
    assert_eq!(
        SecretKey::from_key_file(file.as_bytes(), Some(&long)).err(),
        Some(SignatureError::WrongPassword)
    );
}

/// The minisign key bytes of an unencrypted reference key, to damage.
fn unencrypted_key_bytes() -> Vec<u8> {
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let text = pair.sk.to_box(None).unwrap().into_string();
    STANDARD.decode(text.lines().nth(1).unwrap()).unwrap()
}

fn key_file_of(bytes: &[u8]) -> String {
    format!("untrusted comment: test\n{}\n", STANDARD.encode(bytes))
}

#[test]
fn bld_045_a_damaged_or_foreign_file_is_not_a_key() {
    let good = unencrypted_key_bytes();
    let mut flipped = good.clone();
    flipped[100] ^= 1;
    let mut algorithm = good.clone();
    algorithm[0] = b'X';
    let mut kdf = good.clone();
    kdf[2] = b'Q';
    let mut longer = good.clone();
    longer.push(0);
    let files = [
        key_file_of(&flipped),
        key_file_of(&algorithm),
        key_file_of(&kdf),
        key_file_of(&longer),
        key_file_of(&good[..good.len() - 1]),
        key_file_of(&[]),
        "untrusted comment: only a comment".to_owned(),
        "not base64 at all !".to_owned(),
        STANDARD.encode("no comment line\nRWQ"),
        String::new(),
    ];
    for file in files {
        assert_eq!(
            SecretKey::from_key_file(file.as_bytes(), None).err(),
            Some(SignatureError::Format),
            "{file}"
        );
    }
    assert_eq!(
        SecretKey::from_key_file(&[0xff, 0xfe, 0x00], None).err(),
        Some(SignatureError::Format)
    );
}

#[test]
fn bld_045_a_key_file_asking_for_more_scrypt_work_than_the_cap_is_refused_before_any() {
    let mut bytes = unencrypted_key_bytes();
    bytes[2..4].copy_from_slice(b"Sc");
    // minisign's own N = 2^20 ceiling, then N = 2^21 and p = 32.
    for (ops, mem) in [(1_u64 << 40, 1_u64 << 34), (33_554_432, 33_554_432)] {
        bytes[OPS_AT..MEM_AT].copy_from_slice(&ops.to_le_bytes());
        bytes[MEM_AT..SEALED_AT].copy_from_slice(&mem.to_le_bytes());
        let started = std::time::Instant::now();
        assert_eq!(
            SecretKey::from_key_file(key_file_of(&bytes).as_bytes(), Some(PASSWORD)).err(),
            Some(SignatureError::Format)
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }
}

#[test]
fn r_116_a_public_key_reads_from_the_tauri_value_and_the_minisign_text() {
    let (file, public) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    let from_tauri = PublicKey::from_key_file(&tauri_public_key(&public)).unwrap();
    let from_text = PublicKey::from_key_file(&public.to_box().unwrap().into_string()).unwrap();
    assert_eq!(from_tauri, key.public_key());
    assert_eq!(from_text, key.public_key());
    assert_eq!(
        PublicKey::from_line(&public.to_base64()).unwrap(),
        from_text
    );
    let (_, other) = tauri_key_file(None);
    assert_ne!(
        PublicKey::from_key_file(&tauri_public_key(&other)).unwrap(),
        key.public_key()
    );
}

#[test]
fn r_116_text_that_is_not_a_public_key_is_refused() {
    let (_, public) = tauri_key_file(None);
    let two_keys = format!(
        "{}\n{}\n",
        public.to_box().unwrap().into_string(),
        public.to_base64()
    );
    for text in [
        "REPLACE_WITH_OPERATOR_PUBLIC_KEY",
        "",
        "untrusted comment: x\nnot a key\n",
        &STANDARD.encode("untrusted comment: x\nnot a key\n"),
        &public.to_base64(),
        &two_keys,
    ] {
        assert_eq!(
            PublicKey::from_key_file(text),
            Err(SignatureError::Format),
            "{text}"
        );
    }
}
