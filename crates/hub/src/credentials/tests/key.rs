use std::{io::Write as _, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::{CONTEXT, secret, test_key, vault_from};
use crate::credentials::{Error, Vault};

fn key_file(contents: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(contents).unwrap();
    file
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
