//! The release commands of the workflow, run as the real `anima-hub` against an in-process R2
//! (R-179; BLD-002, BLD-045, BLD-046, BLD-064 to BLD-069, UPD-008, UPD-010): a job signs, and a
//! second job publishes with no hub and no database. Every key and token is made here.

use std::{path::Path, process::Command};

use minisign::KeyPair;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::{
    fake_r2::{FakeR2, hex},
    setup::{
        File, LINUX_FEED, LINUX_STABLE, linux_files, public_key, secret_files, verified_version,
    },
};

const MAC_FEEDS: [&str; 2] = ["darwin-aarch64/latest.json", "darwin-x86_64/latest.json"];

/// The fake bucket, the throwaway secrets and the settings the workflow gives the commands.
pub struct Workflow {
    pub fake: FakeR2,
    pub secrets: tempfile::TempDir,
    pub pair: KeyPair,
}

/// What one run of `anima-hub release` ended with: whether it passed, and all it printed.
pub struct Ran {
    pub passed: bool,
    pub printed: String,
}

impl Workflow {
    pub async fn start() -> Self {
        let (secrets, pair) = secret_files();
        Self {
            fake: FakeR2::start().await,
            secrets,
            pair,
        }
    }

    /// Runs `anima-hub release <verb> <target> <version> <folder>` with the settings of the
    /// job that has `signer`'s key files, and the fake as the bucket.
    pub async fn run(&self, verb: &str, release: (&str, &str), folder: &Path) -> Ran {
        let secrets = self.secrets.path();
        let mut command = Command::new(env!("CARGO_BIN_EXE_anima-hub"));
        command
            .env_clear()
            .args(["release", verb, release.0, release.1])
            .arg(folder)
            .env("ANIMA_UPDATER_PUBLIC_KEY", public_key(&self.pair))
            .env("ANIMA_UPDATER_KEY_FILE", secrets.join("updater_key"))
            .env(
                "ANIMA_UPDATER_KEY_PASSWORD_FILE",
                secrets.join("updater_key_password"),
            )
            .env("SOURCE_DATE_EPOCH", "1888000000")
            .env("ANIMA_RELEASES_PUBLIC_BASE", self.fake.public_base());
        if verb == "publish" {
            command
                .env("ANIMA_RELEASES_ENDPOINT", self.fake.endpoint())
                .env("ANIMA_RELEASES_ACCESS_KEY_ID_FILE", secrets.join("id"))
                .env(
                    "ANIMA_RELEASES_SECRET_ACCESS_KEY_FILE",
                    secrets.join("secret"),
                );
        }
        let output = tokio::task::spawn_blocking(move || command.output().unwrap())
            .await
            .unwrap();
        let mut printed = String::from_utf8_lossy(&output.stdout).into_owned();
        printed.push_str(&String::from_utf8_lossy(&output.stderr));
        Ran {
            passed: output.status.success(),
            printed,
        }
    }

    /// Signs the files of `folder` and publishes them; the publish run.
    pub async fn release(&self, release: (&str, &str), folder: &Path) -> Ran {
        let signed = self.run("sign", release, folder).await;
        assert!(signed.passed, "{}", signed.printed);
        self.run("publish", release, folder).await
    }

    pub fn bytes(&self, object: &str) -> Option<Vec<u8>> {
        self.fake.object(object).map(|object| object.bytes)
    }

    pub fn feed(&self, object: &str) -> Value {
        serde_json::from_slice(&self.bytes(object).expect("the feed is written")).unwrap()
    }

    pub fn writes(&self) -> Vec<String> {
        self.fake.with(|store| store.writes.clone())
    }
}

/// A folder with `files`, as the publish job has it after it took the artifacts.
pub fn folder_of(files: &[File]) -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    for file in files {
        std::fs::write(folder.path().join(&file.name), &file.bytes).unwrap();
    }
    folder
}

/// The files of a macOS job of `version`: the updater file, the DMG and `SHA256SUMS`.
fn mac_files(version: &str) -> Vec<File> {
    let app = File::new("Anima.app.tar.gz", format!("app {version} ").repeat(300));
    let dmg = File::new(
        &format!("Anima_{version}_universal.dmg"),
        format!("dmg {version} ").repeat(300),
    );
    let line = |file: &File| format!("{}  {}\n", hex(&Sha256::digest(&file.bytes)), file.name);
    let sums = format!("{}{}", line(&app), line(&dmg));
    vec![app, dmg, File::new("SHA256SUMS", sums)]
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_066_a_signed_linux_release_is_published_with_the_feed_last() {
    let workflow = Workflow::start().await;
    let files = linux_files("0.2.2");
    let folder = folder_of(&files);

    let published = workflow
        .release(("linux-x86_64", "0.2.2"), folder.path())
        .await;

    assert!(published.passed, "{}", published.printed);
    let app = &files[0];
    let versioned = format!("linux-x86_64/0.2.2/{}", app.name);
    assert_eq!(workflow.bytes(&versioned).as_ref(), Some(&app.bytes));
    assert_eq!(workflow.bytes(LINUX_STABLE).as_ref(), Some(&app.bytes));
    assert!(workflow.bytes("linux-x86_64/0.2.2/SHA256SUMS").is_some());
    let sig = String::from_utf8(workflow.bytes(&format!("{versioned}.sig")).unwrap()).unwrap();
    assert_eq!(
        verified_version(&workflow.pair, &sig, &app.bytes),
        Ok("0.2.2".to_owned())
    );
    let feed = workflow.feed(LINUX_FEED);
    assert_eq!(feed["version"], "0.2.2");
    let entry = &feed["platforms"]["linux-x86_64"];
    assert_eq!(entry["signature"], sig.as_str());
    let url = format!("{}/{versioned}", workflow.fake.public_base());
    assert_eq!(entry["url"], url.as_str());
    assert_eq!(
        workflow.writes().last().map(String::as_str),
        Some(LINUX_FEED)
    );
    let key_file = std::fs::read_to_string(workflow.secrets.path().join("updater_key")).unwrap();
    assert!(!published.printed.contains(&key_file));
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_067_a_macos_release_writes_the_same_feed_to_both_architectures() {
    let workflow = Workflow::start().await;
    let folder = folder_of(&mac_files("0.2.2"));

    let published = workflow
        .release(("darwin-universal", "0.2.2"), folder.path())
        .await;

    assert!(published.passed, "{}", published.printed);
    let [arm, intel] = MAC_FEEDS.map(|feed| workflow.bytes(feed).expect("the feed is written"));
    assert_eq!(arm, intel);
    assert_eq!(workflow.feed(MAC_FEEDS[0])["version"], "0.2.2");
    assert!(workflow.bytes("Anima.dmg").is_some());
    let writes = workflow.writes();
    assert_eq!(writes[writes.len() - 2..], MAC_FEEDS);
}

#[tokio::test(flavor = "multi_thread")]
async fn r_179_a_trial_checks_the_files_and_reaches_no_bucket() {
    let workflow = Workflow::start().await;
    let folder = folder_of(&mac_files("0.2.2"));
    let signed = workflow
        .run("sign", ("darwin-universal", "0.2.2"), folder.path())
        .await;
    assert!(signed.passed, "{}", signed.printed);

    let checked = workflow
        .run("check", ("darwin-universal", "0.2.2"), folder.path())
        .await;

    assert!(checked.passed, "{}", checked.printed);
    assert!(checked.printed.contains("Nothing is published"));
    assert_eq!(workflow.writes(), Vec::<String>::new());

    std::fs::write(folder.path().join("notes.html"), "<script>").unwrap();
    let refused = workflow
        .run("check", ("darwin-universal", "0.2.2"), folder.path())
        .await;
    assert!(!refused.passed);
    assert!(
        refused.printed.contains("notes.html"),
        "{}",
        refused.printed
    );
}
