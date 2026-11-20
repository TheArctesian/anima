//! `anima-hub release record` (task U2; ROL-003 to ROL-005, ROL-010): the job that holds the key
//! writes a record that the check of a machine accepts, and refuses a folder that is not a
//! release of programs.

use std::{collections::BTreeMap, path::Path};

use release_record::{Expected, FileCheck, Program, RECORD_FILE, SIGNATURE_FILE, Target, check};
use updater_signature::PublicKey;

use super::tauri_key;
use crate::fleet::builds::release_command;

const DAEMON: &[u8] = b"anima-daemon for linux";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// The settings of the job `sign` with a throwaway key in `keys`, and its public key.
fn settings(keys: &Path) -> (BTreeMap<String, String>, PublicKey) {
    let (file, public, _) = tauri_key(None);
    std::fs::write(keys.join("key"), file).unwrap();
    std::fs::write(keys.join("password"), "").unwrap();
    let path = |name: &str| keys.join(name).to_string_lossy().into_owned();
    let env = BTreeMap::from([
        ("ANIMA_UPDATER_KEY_FILE".to_owned(), path("key")),
        (
            "ANIMA_UPDATER_KEY_PASSWORD_FILE".to_owned(),
            path("password"),
        ),
        ("ANIMA_UPDATER_PUBLIC_KEY".to_owned(), public.clone()),
        ("SOURCE_DATE_EPOCH".to_owned(), "1800000000".to_owned()),
        ("ANIMA_RELEASE_COMMIT".to_owned(), COMMIT.to_owned()),
        ("ANIMA_RELEASE_REPLACES_FROM".to_owned(), "0.0.0".to_owned()),
    ]);
    (env, PublicKey::from_key_file(&public).unwrap())
}

async fn record(env: &BTreeMap<String, String>, folder: &Path) -> Result<String, String> {
    let folder = folder.to_string_lossy();
    let args = ["record", "linux-x86_64", "0.9.3", &folder];
    release_command(env, &args, &reqwest::Client::new())
        .await
        .map_err(|error| error.to_string())
}

#[tokio::test]
async fn rol_004_rol_010_the_record_that_the_job_writes_passes_the_check_of_a_machine() {
    let (keys, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (env, public) = settings(keys.path());
    std::fs::write(folder.path().join("anima-daemon"), DAEMON).unwrap();
    let said = record(&env, folder.path()).await.unwrap();
    assert_eq!(
        said,
        "Recorded: linux-x86_64 0.9.3 with anima-daemon. release-record.json and its signature are written."
    );
    let bytes = std::fs::read(folder.path().join(RECORD_FILE)).unwrap();
    let signature = std::fs::read_to_string(folder.path().join(SIGNATURE_FILE)).unwrap();
    let expected = Expected {
        version: "0.9.3".parse().unwrap(),
        target: Target::LinuxX8664,
        program: Program::Daemon,
    };
    let checked = check(&bytes, &signature, &public, expected).unwrap();
    assert_eq!(checked.record.commit, COMMIT);
    assert_eq!(checked.record.replaces_from.to_string(), "0.0.0");
    let mut file = FileCheck::new(&checked.file);
    file.update(DAEMON).unwrap();
    file.finish().unwrap();
}

#[tokio::test]
async fn a_folder_with_a_file_that_is_not_a_program_or_with_no_file_is_refused() {
    let (keys, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (env, _) = settings(keys.path());
    assert_eq!(
        record(&env, folder.path()).await,
        Err("the folder holds no program file".to_owned())
    );
    std::fs::write(folder.path().join("Anima.app.tar.gz"), DAEMON).unwrap();
    let refused = record(&env, folder.path()).await.unwrap_err();
    assert_eq!(
        refused,
        "Anima.app.tar.gz in the folder is not the file of a program of Anima"
    );
    assert!(!folder.path().join(RECORD_FILE).exists());
}

#[tokio::test]
async fn the_commit_and_the_lowest_version_are_settings_that_must_be_valid() {
    let (keys, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    std::fs::write(folder.path().join("anima-daemon"), DAEMON).unwrap();
    let (env, _) = settings(keys.path());
    let mut no_commit = env.clone();
    no_commit.remove("ANIMA_RELEASE_COMMIT");
    assert_eq!(
        record(&no_commit, folder.path()).await,
        Err("ANIMA_RELEASE_COMMIT is not set or is not valid".to_owned())
    );
    let mut short = env.clone();
    short.insert("ANIMA_RELEASE_COMMIT".to_owned(), "0123".to_owned());
    assert!(
        record(&short, folder.path())
            .await
            .unwrap_err()
            .starts_with("the release record breaks a rule")
    );
    let mut above = env;
    above.insert("ANIMA_RELEASE_REPLACES_FROM".to_owned(), "0.9.4".to_owned());
    assert!(
        record(&above, folder.path())
            .await
            .unwrap_err()
            .starts_with("the release record breaks a rule")
    );
    assert!(!folder.path().join(RECORD_FILE).exists());
}

#[tokio::test]
async fn rol_008_a_target_with_no_daemon_release_has_no_record() {
    let (keys, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    std::fs::write(folder.path().join("anima-daemon"), DAEMON).unwrap();
    let (env, _) = settings(keys.path());
    let path = folder.path().to_string_lossy();
    let args = ["record", "windows-x86_64", "0.9.3", &path];
    let refused = release_command(&env, &args, &reqwest::Client::new())
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(
        refused,
        "a release record is only for linux-x86_64 and darwin-universal"
    );
    assert!(!folder.path().join(RECORD_FILE).exists());
}
