use std::{fmt, io::Write as _, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use static_assertions::assert_not_impl_any;
use zeroize::Zeroizing;

use super::{Error, Sealed, Secret, Vault};

/// Row context used by tests that do not exercise context binding.
const CONTEXT: &[u8] = b"test/context";

/// Bytes 0x00..=0x1f: a fixed throwaway test key.
fn test_key() -> [u8; 32] {
    std::array::from_fn(|index| u8::try_from(index).unwrap())
}

fn vault_from(key: [u8; 32]) -> Vault {
    Vault::from_key(Zeroizing::new(key))
}

fn other_vault() -> Vault {
    vault_from([0xa5; 32])
}

fn secret(text: &str) -> Secret {
    Secret::from(text.to_owned())
}

fn key_file(contents: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(contents).unwrap();
    file
}

fn with_ciphertext(sealed: &Sealed, ciphertext: Vec<u8>) -> Result<Sealed, Error> {
    Sealed::from_parts(sealed.nonce(), ciphertext, sealed.key_id().to_owned())
}

#[test]
fn sec_003_seal_then_open_returns_plaintext() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("correct horse battery staple"), CONTEXT);
    assert_eq!(
        vault.open(&sealed, CONTEXT).unwrap().expose(),
        b"correct horse battery staple"
    );
}

#[test]
fn sec_003_empty_secret_round_trips() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&Secret::new(Vec::new()), CONTEXT);
    assert_eq!(vault.open(&sealed, CONTEXT).unwrap().expose(), b"");
}

#[test]
fn sec_003_sealing_twice_uses_fresh_nonces_and_ciphertexts() {
    let vault = vault_from(test_key());
    let first = vault.seal(&secret("same"), CONTEXT);
    let second = vault.seal(&secret("same"), CONTEXT);
    assert_ne!(first.nonce(), second.nonce());
    assert_ne!(first.ciphertext(), second.ciphertext());
}

#[test]
fn sec_003_ciphertext_does_not_contain_plaintext() {
    let sealed = vault_from(test_key()).seal(&secret("plain-marker-text"), CONTEXT);
    let marker = b"plain-marker-text";
    assert!(
        !sealed
            .ciphertext()
            .windows(marker.len())
            .any(|w| w == marker)
    );
}

#[test]
fn sec_003_wrong_key_fails() {
    let sealed = vault_from(test_key()).seal(&secret("s"), CONTEXT);
    assert!(matches!(
        other_vault().open(&sealed, CONTEXT),
        Err(Error::WrongKey)
    ));
}

#[test]
fn sec_003_same_key_id_with_a_different_key_fails_to_decrypt() {
    let sealed = vault_from(test_key()).seal(&secret("s"), CONTEXT);
    let other = other_vault();
    let forged = Sealed::from_parts(
        sealed.nonce(),
        sealed.ciphertext().to_vec(),
        other.seal(&secret("x"), CONTEXT).key_id().to_owned(),
    )
    .unwrap();
    assert!(matches!(other.open(&forged, CONTEXT), Err(Error::Decrypt)));
}

#[test]
fn sec_003_changed_nonce_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), CONTEXT);
    let mut nonce = sealed.nonce().to_vec();
    nonce[0] ^= 0x01;
    let changed = Sealed::from_parts(
        &nonce,
        sealed.ciphertext().to_vec(),
        sealed.key_id().to_owned(),
    )
    .unwrap();
    assert!(matches!(vault.open(&changed, CONTEXT), Err(Error::Decrypt)));
}

#[test]
fn sec_003_changed_ciphertext_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("secret text"), CONTEXT);
    for bit in 0..sealed.ciphertext().len() * 8 {
        let mut ciphertext = sealed.ciphertext().to_vec();
        ciphertext[bit / 8] ^= 1 << (bit % 8);
        let changed = with_ciphertext(&sealed, ciphertext).unwrap();
        assert!(
            matches!(vault.open(&changed, CONTEXT), Err(Error::Decrypt)),
            "bit {bit}"
        );
    }
}

#[test]
fn sec_003_truncated_ciphertext_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("secret text"), CONTEXT);
    for len in 0..sealed.ciphertext().len() {
        let truncated = sealed.ciphertext()[..len].to_vec();
        match with_ciphertext(&sealed, truncated) {
            Ok(changed) => assert!(matches!(vault.open(&changed, CONTEXT), Err(Error::Decrypt))),
            Err(error) => assert!(matches!(error, Error::Malformed)),
        }
    }
}

#[test]
fn sec_003_changed_key_id_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), CONTEXT);
    let changed = Sealed::from_parts(
        sealed.nonce(),
        sealed.ciphertext().to_vec(),
        "0000000000000000".to_owned(),
    )
    .unwrap();
    assert!(matches!(
        vault.open(&changed, CONTEXT),
        Err(Error::WrongKey)
    ));
}

#[test]
fn sec_003_other_context_fails() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), b"device-a/ssh_password");
    let opened = vault.open(&sealed, b"device-a/ssh_password");
    assert_eq!(opened.unwrap().expose(), b"s");
    for context in [
        &b"device-b/ssh_password"[..],
        b"device-a/ssh_passphrase",
        b"",
    ] {
        assert!(matches!(vault.open(&sealed, context), Err(Error::Decrypt)));
    }
}

#[test]
fn sec_003_empty_context_is_not_a_prefix_of_another() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), b"");
    assert_eq!(vault.open(&sealed, b"").unwrap().expose(), b"s");
    assert!(matches!(vault.open(&sealed, b"\0"), Err(Error::Decrypt)));
}

#[test]
fn sec_003_sealed_bytes_round_trip() {
    let vault = vault_from(test_key());
    let sealed = vault.seal(&secret("s"), CONTEXT);
    let parsed = Sealed::from_bytes(&sealed.to_bytes()).unwrap();
    assert_eq!(parsed, sealed);
    assert_eq!(vault.open(&parsed, CONTEXT).unwrap().expose(), b"s");
}

#[test]
fn sec_003_malformed_sealed_bytes_are_rejected() {
    let good = vault_from(test_key())
        .seal(&secret("s"), CONTEXT)
        .to_bytes();
    let mut bad_version = good.clone();
    bad_version[0] = 0x02;
    let mut long_id = good.clone();
    long_id[1] = 0xff;
    let mut not_utf8 = good.clone();
    not_utf8[2] = 0xff;
    let short_nonce = good[..2 + 16 + 23].to_vec();
    let short_ciphertext = good[..2 + 16 + 24 + 15].to_vec();
    let cases: [&[u8]; 8] = [
        &[],
        &[0x01],
        &[0x01, 0x00],
        &bad_version,
        &long_id,
        &not_utf8,
        &short_nonce,
        &short_ciphertext,
    ];
    for case in cases {
        assert!(matches!(Sealed::from_bytes(case), Err(Error::Malformed)));
    }
}

/// Property: every prefix and every single-byte change of a valid value either fails to
/// parse or fails to open; nothing panics and nothing opens to other plaintext.
#[test]
fn sec_003_any_altered_sealed_bytes_never_open() {
    let vault = vault_from(test_key());
    let good = vault.seal(&secret("property"), CONTEXT).to_bytes();
    for len in 0..good.len() {
        match Sealed::from_bytes(&good[..len]) {
            Ok(sealed) => assert!(vault.open(&sealed, CONTEXT).is_err(), "prefix {len}"),
            Err(error) => assert!(matches!(error, Error::Malformed), "prefix {len}"),
        }
    }
    for index in 0..good.len() {
        for flip in [0x01_u8, 0x80, 0xff] {
            let mut bytes = good.clone();
            bytes[index] ^= flip;
            if let Ok(sealed) = Sealed::from_bytes(&bytes) {
                assert!(
                    vault.open(&sealed, CONTEXT).is_err(),
                    "byte {index} flip {flip:#x}"
                );
            }
        }
    }
}

#[test]
fn sec_003_from_parts_rejects_a_short_nonce() {
    let result = Sealed::from_parts(&[0; 23], vec![0; 16], "0123456789abcdef".to_owned());
    assert!(matches!(result, Err(Error::Malformed)));
}

#[test]
fn sec_004_key_id_is_pinned_for_a_fixed_key() {
    let sealed = vault_from(test_key()).seal(&secret("s"), CONTEXT);
    assert_eq!(sealed.key_id(), "2407973bf9f2a329");
}

#[test]
fn sec_004_key_from_file() {
    let encoded = STANDARD.encode(test_key());
    for contents in [
        encoded.clone(),
        format!("{encoded}\n"),
        format!("  {encoded}\r\n"),
    ] {
        let file = key_file(contents.as_bytes());
        let from_file = Vault::from_key_file(file.path()).unwrap();
        let sealed = from_file.seal(&secret("s"), CONTEXT);
        assert_eq!(sealed.key_id(), "2407973bf9f2a329");
        assert_eq!(
            vault_from(test_key())
                .open(&sealed, CONTEXT)
                .unwrap()
                .expose(),
            b"s"
        );
    }
}

fn assert_reveals_nothing(error: &Error, path: &Path, key_text: &str) {
    let message = error.to_string();
    let file_name = path.file_name().unwrap().to_string_lossy();
    assert!(!message.contains(&*path.to_string_lossy()), "{message}");
    assert!(!message.contains(&*file_name), "{message}");
    assert!(
        key_text.is_empty() || !message.contains(key_text),
        "{message}"
    );
}

#[test]
fn sec_004_bad_key_files_fail_without_revealing_path_or_key() {
    let short = STANDARD.encode([7_u8; 31]);
    let long = STANDARD.encode([7_u8; 33]);
    let not_base64 = "not*base64*key*text*not*base64*key*text*abc=".to_owned();
    for text in [short, long, not_base64, String::new(), "   \n".to_owned()] {
        let file = key_file(text.as_bytes());
        let error = Vault::from_key_file(file.path()).unwrap_err();
        assert!(matches!(error, Error::KeyFormat), "{text:?}");
        assert_reveals_nothing(&error, file.path(), text.trim());
    }
}

#[test]
fn sec_004_missing_key_file_fails_without_revealing_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent-anima-key");
    let error = Vault::from_key_file(&path).unwrap_err();
    assert!(matches!(
        error,
        Error::KeyFile(std::io::ErrorKind::NotFound)
    ));
    assert_reveals_nothing(&error, &path, "");
}

#[test]
fn log_007_secret_has_no_format_traits() {
    assert_not_impl_any!(Secret: fmt::Debug, fmt::Display, serde::Serialize, Clone, PartialEq);
}

#[test]
fn log_007_vault_debug_shows_only_the_key_id() {
    let key = test_key();
    let printed = format!("{:?}", vault_from(key));
    assert_eq!(printed, r#"Vault { key_id: "2407973bf9f2a329", .. }"#);
    assert!(!printed.contains(&STANDARD.encode(key)));
}
