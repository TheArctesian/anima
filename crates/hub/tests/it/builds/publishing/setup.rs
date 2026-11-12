//! A hub with a release bucket on the fake R2 and a throwaway updater key, a builder that
//! uploads through the hub's presigned URLs, and the files, sums and unsigned feed of one job
//! (R-116). No token or key here is real; the token and key files are in a temporary folder
//! (SEC-004).

use std::{io::Cursor, sync::Arc};

use anima_hub::{
    builds::{self, Bucket, BuildQueue, Updates},
    clock::Clock,
    config::Releases,
    events::Bus,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, TimeDelta, Utc};
use minisign::{KeyPair, SignatureBox};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use super::fake_r2::{BUCKET, FakeR2, KEY_ID, SECRET, hex};
use crate::{
    builds::harness::{COMMIT, data, operator},
    events::client::TestClock,
    hub::{Hub, dependencies, start},
};

pub const LINUX_FEED: &str = "linux-x86_64/latest.json";
/// The stable names sit at the top of the bucket (Section 3.10.4).
pub const LINUX_STABLE: &str = "Anima-x86_64.AppImage";

/// When a job uploaded at `start()` may be published: its upload URLs expired 15 minutes after
/// it and the hub waits 2 more for clock skew (D-28).
pub fn ready() -> DateTime<Utc> {
    start() + TimeDelta::minutes(20)
}

/// A job a builder holds, and the builder's token.
pub struct Held {
    pub job: String,
    pub token: String,
}

/// One file of a job, with its bytes.
#[derive(Clone)]
pub struct File {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl File {
    pub fn new(name: &str, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.to_owned(),
            bytes: bytes.into(),
        }
    }

    /// As `BuildFileInput`.
    pub fn input(&self) -> Value {
        json!({
            "name": self.name,
            "sha256": hex(&Sha256::digest(&self.bytes)),
            "size": self.bytes.len(),
        })
    }
}

/// The password of the throwaway updater key.
pub const KEY_PASSWORD: &str = "a throwaway key password";

/// The hub, the fake bucket and the updater key pair of one test.
pub struct Publishing {
    pub pool: PgPool,
    pub fake: FakeR2,
    pub bucket: Arc<Bucket>,
    pub hub: Hub,
    pub operator: String,
    pub pair: KeyPair,
    /// The clock of the hub and of the publisher.
    pub clock: Arc<TestClock>,
    http: reqwest::Client,
    _tokens: tempfile::TempDir,
}

/// Where the hub reads the throwaway R2 token and the throwaway updater key, sealed with
/// [`KEY_PASSWORD`] as `tauri signer generate` writes it; and the key pair.
pub fn secret_files() -> (tempfile::TempDir, KeyPair) {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("id"), format!("{KEY_ID}\n")).unwrap();
    std::fs::write(folder.path().join("secret"), SECRET).unwrap();
    let pair = KeyPair::generate_encrypted_keypair(Some(KEY_PASSWORD.to_owned())).unwrap();
    let key_file = STANDARD.encode(pair.sk.to_box(None).unwrap().into_string());
    std::fs::write(folder.path().join("updater_key"), key_file).unwrap();
    std::fs::write(folder.path().join("updater_key_password"), KEY_PASSWORD).unwrap();
    (folder, pair)
}

/// The release settings of the fake at `fake`, the secrets in `folder` and `public` as
/// `ANIMA_UPDATER_PUBLIC_KEY`.
pub fn releases(fake: &FakeR2, folder: &std::path::Path, public: Option<String>) -> Releases {
    Releases {
        endpoint: fake.endpoint(),
        bucket: BUCKET.to_owned(),
        public_base: fake.public_base(),
        access_key_id_file: folder.join("id"),
        secret_access_key_file: folder.join("secret"),
        updater_public_key: public,
        updater_key_file: folder.join("updater_key"),
        updater_key_password_file: folder.join("updater_key_password"),
    }
}

/// `plugins.updater.pubkey` of `pair`.
pub fn public_key(pair: &KeyPair) -> String {
    STANDARD.encode(pair.pk.to_box().unwrap().into_string())
}

impl Publishing {
    /// A hub whose bucket opened the throwaway updater key from its files.
    pub async fn start(pool: &PgPool) -> Self {
        let fake = FakeR2::start().await;
        fake.with(|store| store.now = Some(start()));
        let (tokens, pair) = secret_files();
        let releases = releases(&fake, tokens.path(), Some(public_key(&pair)));
        let bucket = Arc::new(builds::open_bucket(&releases).unwrap().unwrap());
        let operator = operator(pool).await;
        let clock = Arc::new(TestClock::at(start()));
        let mut parts = dependencies(pool, clock.clone(), Bus::new(16));
        parts.bucket = Some(bucket.clone());
        Self {
            pool: pool.clone(),
            hub: Hub::serving(pool, parts),
            fake,
            bucket,
            operator,
            pair,
            clock,
            http: reqwest::Client::new(),
            _tokens: tokens,
        }
    }

    /// Sets the hub's clock and the fake's to `now`.
    pub fn at(&self, now: DateTime<Utc>) {
        self.clock.set(now);
        self.fake.with(|store| store.now = Some(now));
    }

    /// The build queue of this hub's database, with throwaway announcers.
    pub fn queue<'a>(&'a self, bus: &'a Bus, updates: &'a Updates) -> BuildQueue<'a> {
        BuildQueue {
            db: &self.pool,
            bus,
            updates,
        }
    }

    /// Checks, with the reference minisign, that the hub signed `app` for `version`: the `.sig`
    /// beside it and the signature of each entry of the Linux feed are the same text, over
    /// exactly `app`'s bytes, with `version` in the trusted comment (BLD-046, R-116).
    pub fn assert_signed(&self, version: &str, app: &File) {
        let object = format!("linux-x86_64/{version}/{}.sig", app.name);
        let sig = self.fake.object(&object).expect("the .sig is written");
        let sig = String::from_utf8(sig.bytes).unwrap();
        let feed: Value =
            serde_json::from_slice(&self.fake.object(LINUX_FEED).unwrap().bytes).unwrap();
        assert_eq!(feed["platforms"]["linux-x86_64"]["signature"], sig.as_str());
        assert_eq!(
            verified_version(&self.pair, &sig, &app.bytes),
            Ok(version.to_owned())
        );
    }

    pub fn public(&self, path: &str) -> String {
        format!("{}/{path}", self.fake.public_base())
    }

    /// Enrols a builder for `platform`, which claims the oldest job it can build and takes it to
    /// `UPLOADING`.
    pub async fn uploading(&self, name: &str, platform: &str) -> Held {
        let enrol = format!(
            "mutation($name: String!) {{ enrolBuilder(name: $name, platform: {platform}) \
             {{ token builder {{ id }} }} }}"
        );
        let enrolled = data(
            self.hub
                .post(Some(&self.operator), &enrol, json!({ "name": name }))
                .await,
        );
        let token = enrolled["enrolBuilder"]["token"]
            .as_str()
            .unwrap()
            .to_owned();
        let claim = format!("mutation {{ claimBuildJob(platform: {platform}) {{ id }} }}");
        let claimed = data(self.hub.post(Some(&token), &claim, json!({})).await);
        let job = claimed["claimBuildJob"]["id"].as_str().unwrap().to_owned();
        for state in ["BUILDING", "UPLOADING"] {
            let report = "mutation($id: ID!, $state: BuildJobState) { \
                          reportBuildJob(id: $id, state: $state, logLines: []) { id } }";
            let variables = json!({ "id": job, "state": state });
            data(self.hub.post(Some(&token), report, variables).await);
        }
        Held { job, token }
    }

    /// Creates a release of `version` for `targets`.
    pub async fn release(&self, version: &str, targets: &str) {
        let create = format!(
            "mutation($version: String!, $commit: String!) {{ \
             createRelease(version: $version, commit: $commit, targets: [{targets}]) {{ id }} }}"
        );
        let variables = json!({ "version": version, "commit": COMMIT });
        data(
            self.hub
                .post(Some(&self.operator), &create, variables)
                .await,
        );
    }

    /// `buildUploadUrls` for `files` of the job `held` holds; the whole response.
    pub async fn grant(&self, held: &Held, files: &[File]) -> Value {
        let query = "mutation($id: ID!, $files: [BuildFileInput!]!) { \
                     buildUploadUrls(id: $id, files: $files) { name url expiresAt \
                     headers { name value } } }";
        let inputs: Vec<Value> = files.iter().map(File::input).collect();
        let variables = json!({ "id": held.job, "files": inputs });
        self.hub.post(Some(&held.token), query, variables).await
    }

    /// A `PUT` of `bytes` to the presigned `upload`, with its headers; the status.
    pub async fn put(&self, upload: &Value, bytes: &[u8]) -> u16 {
        let mut request = self
            .http
            .put(upload["url"].as_str().unwrap())
            .body(bytes.to_vec());
        for header in upload["headers"].as_array().unwrap() {
            let name = header["name"].as_str().unwrap();
            if name != "content-length" {
                request = request.header(name, header["value"].as_str().unwrap());
            }
        }
        request.send().await.unwrap().status().as_u16()
    }

    /// Grants, uploads every file and reports them with `feed`; the report's response.
    pub async fn upload(&self, held: &Held, files: &[File], feed: &Value) -> Value {
        let granted = data(self.grant(held, files).await);
        let urls = granted["buildUploadUrls"].as_array().unwrap();
        for (upload, file) in urls.iter().zip(files) {
            assert_eq!(upload["name"], file.name.as_str());
            assert_eq!(self.put(upload, &file.bytes).await, 200, "{}", file.name);
        }
        self.report(held, files, feed).await
    }

    pub async fn report(&self, held: &Held, files: &[File], feed: &Value) -> Value {
        let query = "mutation($id: ID!, $files: [BuildFileInput!]!, $feed: String!) { \
                     reportBuildUploaded(id: $id, files: $files, feed: $feed) { state leaseEndsAt } }";
        let inputs: Vec<Value> = files.iter().map(File::input).collect();
        let variables = json!({ "id": held.job, "files": inputs, "feed": feed.to_string() });
        self.hub.post(Some(&held.token), query, variables).await
    }

    /// One publisher sweep with both clocks at `now`; how many tries ended.
    pub async fn sweep(&self, now: DateTime<Utc>) -> usize {
        self.at(now);
        let (bus, updates) = (Bus::new(16), Updates::new());
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let clock: &dyn Clock = self.clock.as_ref();
        builds::publish_ready(self.queue(&bus, &updates), &self.bucket, &http, clock)
            .await
            .unwrap()
    }

    /// The job's state and failure reason.
    pub async fn job(&self, held: &Held) -> (String, Option<String>) {
        sqlx::query_as("SELECT state, failure_reason FROM builds.jobs WHERE id = $1::uuid")
            .bind(&held.job)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

/// Waits until a request waits at the fake's gate.
pub async fn until_waiting(publishing: &Publishing) {
    let waiting = async {
        while publishing.fake.with(|store| store.waiting) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), waiting)
        .await
        .unwrap();
}

/// The version in the trusted comment of the `.sig` text `sig`, once the reference minisign
/// accepts it over `bytes` with `pair`'s public key; why not, otherwise.
pub fn verified_version(pair: &KeyPair, sig: &str, bytes: &[u8]) -> Result<String, String> {
    let text = STANDARD.decode(sig.trim()).map_err(|e| e.to_string())?;
    let text = String::from_utf8(text).map_err(|e| e.to_string())?;
    let parsed = SignatureBox::from_string(&text).map_err(|e| e.to_string())?;
    minisign::verify(&pair.pk, &parsed, Cursor::new(bytes), true, false, false)
        .map_err(|e| e.to_string())?;
    let comment = parsed.trusted_comment().map_err(|e| e.to_string())?;
    comment
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"))
        .map(str::to_owned)
        .ok_or_else(|| "no version".to_owned())
}

/// The files of a Linux job of `version`: the `AppImage` and `SHA256SUMS`.
pub fn linux_files(version: &str) -> Vec<File> {
    let app = File::new(
        &format!("Anima_{version}_amd64.AppImage"),
        format!("AppImage {version} ").repeat(400),
    );
    let sums = format!("{}  {}\n", hex(&Sha256::digest(&app.bytes)), app.name);
    vec![app, File::new("SHA256SUMS", sums)]
}

/// The unsigned feed a builder proposes for a Linux job of `version` naming `app` (R-116).
pub fn linux_feed(publishing: &Publishing, version: &str, app: &File) -> Value {
    let url = publishing.public(&format!("linux-x86_64/{version}/{}", app.name));
    json!({
        "version": version,
        "notes": format!("Anima {version}"),
        "pub_date": "2030-05-05T12:00:00Z",
        "platforms": { "linux-x86_64": { "url": url } },
    })
}

/// A Linux job of `version` uploaded with a good feed, ready to publish.
pub async fn uploaded_linux(publishing: &Publishing, version: &str) -> (Held, Vec<File>) {
    publishing.release(version, "LINUX_X86_64").await;
    let held = publishing.uploading("pop-os", LINUX).await;
    let files = linux_files(version);
    let feed = linux_feed(publishing, version, &files[0]);
    data(publishing.upload(&held, &files, &feed).await);
    (held, files)
}

pub const LINUX: &str = "{ os: LINUX, arch: X86_64 }";
pub const MACOS: &str = "{ os: MACOS, arch: AARCH64 }";
