//! The hub refuses to start without a usable credential key, naming only the path (SEC-004).

use std::{io::Write as _, path::Path, process::Command};

/// Runs `anima-hub` with `key_file` as `ANIMA_KEY_FILE`; returns (success, stderr).
fn start_with_key(key_file: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_anima-hub"))
        .env_clear()
        .env("ANIMA_KEY_FILE", key_file)
        .env("ANIMA_LOG", "error")
        .output()
        .unwrap();
    (
        output.status.success(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn sec_004_a_missing_key_file_stops_start_up_naming_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("anima_key");
    let (ok, stderr) = start_with_key(&missing);
    assert!(!ok);
    assert_eq!(
        stderr.trim(),
        format!(
            "anima-hub: cannot load the credential key from {}: the credential key file cannot \
             be read (entity not found)",
            missing.display()
        )
    );
}

#[test]
fn sec_004_a_bad_key_file_stops_start_up_without_its_bytes() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "not-a-key-but-secret-looking-text").unwrap();
    let (ok, stderr) = start_with_key(file.path());
    assert!(!ok);
    assert!(
        stderr.contains(&file.path().display().to_string()),
        "{stderr}"
    );
    assert!(
        stderr.contains("does not hold a base64-encoded 32-byte key"),
        "{stderr}"
    );
    assert!(!stderr.contains("secret-looking"), "{stderr}");
}
