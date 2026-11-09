//! Every key here is made in the test by the reference minisign crate (the one tauri-cli uses);
//! no real key file is read (UPD-012).

use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign::{KeyPair, SignatureBox};

mod sealing;

use super::{
    UpdaterKeyError,
    sealed::scrypt_cost,
    secret::SecretKey,
    signature::{Signed, sign, verify},
    state_of,
};

const PASSWORD: &str = "correct horse";
const DATA: &[u8] = b"an AppImage, as far as signing cares";

/// A reference key pair: the secret key file as `tauri signer generate` writes it (base64 of the
/// minisign text) and the reference public key.
fn tauri_key_file(password: Option<&str>) -> (String, minisign::PublicKey) {
    let pair = match password {
        Some(password) => KeyPair::generate_encrypted_keypair(Some(password.to_owned())).unwrap(),
        None => KeyPair::generate_unencrypted_keypair().unwrap(),
    };
    let text = pair.sk.to_box(None).unwrap().into_string();
    (STANDARD.encode(text), pair.pk)
}

fn signed(version: &str) -> Signed<'_> {
    Signed {
        timestamp: 1_700_000_000,
        file_name: "Anima_0.1.12_amd64.AppImage",
        version,
    }
}

#[test]
fn bld_045_an_encrypted_tauri_key_file_opens_with_its_password() {
    let (file, public) = tauri_key_file(Some(PASSWORD));
    let key = SecretKey::from_key_file(file.as_bytes(), Some(PASSWORD)).unwrap();
    assert_eq!(key.public_key().line(), public.to_base64());
}

#[test]
fn bld_045_a_key_file_encrypted_with_no_password_opens_without_one() {
    let (file, public) = tauri_key_file(Some(""));
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    assert_eq!(key.public_key().line(), public.to_base64());
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
        Some(UpdaterKeyError::WrongPassword)
    );
    assert_eq!(
        SecretKey::from_key_file(file.as_bytes(), None).err(),
        Some(UpdaterKeyError::PasswordNeeded)
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
            Some(UpdaterKeyError::Format),
            "{file}"
        );
    }
    assert_eq!(
        SecretKey::from_key_file(&[0xff, 0xfe, 0x00], None).err(),
        Some(UpdaterKeyError::Format)
    );
}

#[test]
fn bld_045_scrypt_cost_matches_minisign_and_refuses_crafted_limits() {
    // The minisign crate's defaults (what tauri-cli makes), then the minisign tool's.
    assert_eq!(scrypt_cost(1_048_576, 33_554_432), Some((15, 1)));
    assert_eq!(scrypt_cost(33_554_432, 1_073_741_824), Some((20, 1)));
    assert_eq!(scrypt_cost(1 << 40, 1 << 34), None);
    assert_eq!(scrypt_cost(u64::MAX, 33_554_432), None);
    assert_eq!(scrypt_cost(0, 0), None);
    // p = 32: more parallel work than a real key file asks for (C-4).
    assert_eq!(scrypt_cost(33_554_432, 33_554_432), None);
}

#[test]
fn bld_046_a_signature_verifies_for_its_version_and_no_other() {
    let (file, _) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    let signature = sign(&key, DATA, &signed("0.1.12")).unwrap();
    let public = key.public_key();
    assert_eq!(verify(&public, DATA, &signature, "0.1.12"), Ok(()));
    assert_eq!(
        verify(&public, DATA, &signature, "0.1.13"),
        Err(UpdaterKeyError::SignedVersion)
    );
    assert_eq!(
        verify(&public, &b"another file"[..], &signature, "0.1.12"),
        Err(UpdaterKeyError::Signature)
    );
    let (other_file, _) = tauri_key_file(None);
    let other = SecretKey::from_key_file(other_file.as_bytes(), None).unwrap();
    assert_eq!(
        verify(&other.public_key(), DATA, &signature, "0.1.12"),
        Err(UpdaterKeyError::Signature)
    );
}

#[test]
fn bld_046_the_reference_minisign_accepts_the_signature_and_reads_the_version() {
    let (file, public) = tauri_key_file(Some(PASSWORD));
    let key = SecretKey::from_key_file(file.as_bytes(), Some(PASSWORD)).unwrap();
    let signature = sign(&key, DATA, &signed("0.1.12")).unwrap();
    let text = String::from_utf8(STANDARD.decode(&signature).unwrap()).unwrap();
    let parsed = SignatureBox::from_string(&text).unwrap();
    minisign::verify(&public, &parsed, Cursor::new(DATA), true, false, false).unwrap();
    // The trusted comment as signed-version.ts and tauri-plugin-updater read it.
    assert_eq!(
        parsed.trusted_comment().unwrap(),
        "timestamp:1700000000\tfile:Anima_0.1.12_amd64.AppImage\tversion:0.1.12"
    );
    assert!(text.starts_with("untrusted comment: signature from tauri secret key\n"));
}

#[test]
fn bld_046_a_trusted_comment_field_cannot_hold_a_tab_or_a_line_break() {
    let (file, _) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    for version in ["0.1.12\tversion:9.9.9", "0.1.12\n", ""] {
        assert_eq!(
            sign(&key, DATA, &signed(version)).err(),
            Some(UpdaterKeyError::Signature)
        );
    }
}

#[test]
fn bld_045_the_web_view_sees_the_public_key_and_whether_it_matches_the_app() {
    let (file, public) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    let line = key.public_key().line();
    assert_eq!(line, public.to_base64());
    let state = state_of(Some(line.clone()), true, Some(&line));
    assert!(state.present && state.unlocked);
    assert_eq!(state.public_key.as_deref(), Some(line.as_str()));
    assert_eq!(state.matches_app, Some(true));
    let other = state_of(Some(line.clone()), false, Some("RWQother"));
    assert_eq!((other.matches_app, other.unlocked), (Some(false), false));
    let missing = state_of(None, true, Some(&line));
    assert_eq!(
        (missing.present, missing.unlocked, missing.matches_app),
        (false, false, None)
    );
    let json = serde_json::to_string(&state).unwrap();
    let seed = STANDARD.encode(&key.sealed_body()[8..40]);
    assert!(!json.contains(&seed));
}

#[test]
fn log_007_errors_carry_only_a_code_and_fixed_text() {
    static_assertions::assert_not_impl_any!(SecretKey: std::fmt::Debug, std::fmt::Display, Clone);
    let json = serde_json::to_value(UpdaterKeyError::WrongPassword).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "code": "password",
            "message": "the password does not open the key file"
        })
    );
}
