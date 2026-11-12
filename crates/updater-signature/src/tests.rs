//! Every key here is made in the test by the reference minisign crate (the one tauri-cli uses);
//! no real key file is read (UPD-012).

use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign::{KeyPair, SignatureBox};

mod key_files;
mod parsers;

use crate::{
    Prehash, PublicKey, SecretKey, SignatureCheck, SignatureError, Signed, sealed::scrypt_cost,
    verify,
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

/// `plugins.updater.pubkey` of the reference public key: base64 of its key file.
fn tauri_public_key(public: &minisign::PublicKey) -> String {
    STANDARD.encode(public.to_box().unwrap().into_string())
}

fn signed(version: &str) -> Signed<'_> {
    Signed {
        timestamp: 1_700_000_000,
        file_name: "Anima_0.1.12_amd64.AppImage",
        version,
    }
}

fn digest(data: &[u8]) -> [u8; 64] {
    let mut hash = Prehash::new();
    hash.update(data);
    hash.finish()
}

fn sign(key: &SecretKey, data: &[u8], signed: &Signed<'_>) -> Result<String, SignatureError> {
    key.sign(&digest(data), signed)
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
        Err(SignatureError::SignedVersion)
    );
    assert_eq!(
        verify(&public, &b"another file"[..], &signature, "0.1.12"),
        Err(SignatureError::Signature)
    );
    let (other_file, _) = tauri_key_file(None);
    let other = SecretKey::from_key_file(other_file.as_bytes(), None).unwrap();
    assert_eq!(
        verify(&other.public_key(), DATA, &signature, "0.1.12"),
        Err(SignatureError::Signature)
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
    // The trusted comment as tauri-plugin-updater reads it.
    assert_eq!(
        parsed.trusted_comment().unwrap(),
        "timestamp:1700000000\tfile:Anima_0.1.12_amd64.AppImage\tversion:0.1.12"
    );
    assert!(text.starts_with("untrusted comment: signature from tauri secret key\n"));
}

#[test]
fn bld_046_the_same_key_digest_and_comment_sign_the_same_text() {
    let (file, _) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    let first = sign(&key, DATA, &signed("0.1.12")).unwrap();
    assert_eq!(sign(&key, DATA, &signed("0.1.12")).unwrap(), first);
    let later = Signed {
        timestamp: 1_700_000_001,
        ..signed("0.1.12")
    };
    assert_ne!(sign(&key, DATA, &later).unwrap(), first);
}

#[test]
fn bld_046_a_reference_signature_is_checked_as_the_updater_checks_it() {
    let pair = KeyPair::generate_unencrypted_keypair().unwrap();
    let public = PublicKey::from_key_file(&tauri_public_key(&pair.pk)).unwrap();
    let sign_for = |comment: &str| {
        let signature = minisign::sign(
            Some(&pair.pk),
            &pair.sk,
            Cursor::new(DATA),
            Some(comment),
            None,
        )
        .unwrap();
        STANDARD.encode(signature.into_string())
    };
    let good = sign_for("timestamp:1\tfile:Anima\tversion:0.1.12");
    assert_eq!(verify(&public, DATA, &good, "0.1.12"), Ok(()));
    // Two version fields: the updater would read the first; the check takes neither.
    let doubled = sign_for("timestamp:1\tversion:0.1.12\tversion:0.1.13");
    assert_eq!(
        verify(&public, DATA, &doubled, "0.1.12"),
        Err(SignatureError::SignedVersion)
    );
    let check = SignatureCheck::new(&public, &good).unwrap();
    let mut stream = check.stream().unwrap();
    stream.update(&DATA[..5]);
    stream.update(&DATA[5..]);
    assert_eq!(stream.finish("0.1.12"), Ok(()));
}

#[test]
fn bld_046_a_trusted_comment_field_cannot_hold_a_tab_or_a_line_break() {
    let (file, _) = tauri_key_file(None);
    let key = SecretKey::from_key_file(file.as_bytes(), None).unwrap();
    for version in ["0.1.12\tversion:9.9.9", "0.1.12\n", ""] {
        assert_eq!(
            sign(&key, DATA, &signed(version)).err(),
            Some(SignatureError::Signature)
        );
    }
    let no_name = Signed {
        file_name: "a\rb",
        ..signed("0.1.12")
    };
    assert_eq!(
        sign(&key, DATA, &no_name).err(),
        Some(SignatureError::Signature)
    );
}

#[test]
fn log_007_a_secret_key_has_no_debug_display_or_clone_and_errors_are_fixed_text() {
    static_assertions::assert_not_impl_any!(SecretKey: std::fmt::Debug, std::fmt::Display, Clone);
    assert_eq!(
        SignatureError::WrongPassword.to_string(),
        "the password does not open the key file"
    );
    assert_eq!(format!("{:?}", SignatureError::Format), "Format");
}
