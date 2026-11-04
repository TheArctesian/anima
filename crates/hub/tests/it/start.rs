//! The hub refuses to start without a usable credential key, naming only the path (SEC-004),
//! and as a database role that could change the audit log (R-016, LOG-006).

use std::{io::Write as _, path::Path, process::Command};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::PgPool;

/// Runs `anima-hub` with `key_file` as `ANIMA_KEY_FILE` and the extra variables; returns
/// (success, stderr).
fn start_with(key_file: &Path, extra: &[(&str, &Path)]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_anima-hub"))
        .env_clear()
        .env("ANIMA_KEY_FILE", key_file)
        .env("ANIMA_LOG", "error")
        .envs(extra.iter().copied())
        .output()
        .unwrap();
    (
        output.status.success(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn start_with_key(key_file: &Path) -> (bool, String) {
    start_with(key_file, &[])
}

/// `DATABASE_URL` (the throwaway test server) with its database replaced by `database`.
fn url_for(database: &str) -> String {
    let url = std::env::var("DATABASE_URL").expect("tests run with DATABASE_URL (R-009)");
    let (base, query) = url
        .split_once('?')
        .map_or((url.as_str(), None), |(b, q)| (b, Some(q)));
    let server = base.rsplit_once('/').map_or(base, |(server, _)| server);
    match query {
        Some(query) => format!("{server}/{database}?{query}"),
        None => format!("{server}/{database}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn log_006_r_016_start_up_refuses_a_superuser_database_role(pool: PgPool) {
    let database = pool.connect_options().get_database().unwrap().to_owned();
    let mut url_file = tempfile::NamedTempFile::new().unwrap();
    write!(url_file, "{}", url_for(&database)).unwrap();
    let mut key_file = tempfile::NamedTempFile::new().unwrap();
    write!(key_file, "{}", STANDARD.encode([3_u8; 32])).unwrap();

    let (ok, stderr) = start_with(
        key_file.path(),
        &[("ANIMA_DATABASE_URL_FILE", url_file.path())],
    );
    assert!(!ok);
    assert!(stderr.contains("is a superuser"), "{stderr}");
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
