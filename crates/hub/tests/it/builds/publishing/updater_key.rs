//! The hub's updater key at start and its `.sig` on a retry (R-116, SEC-004, LOG-007, D-25): a
//! key that does not open with its password, or opens to another public key, turns publishing
//! off with a fixed warning that names no key text, and the hub still has its other parts; a
//! `.sig` write whose answer was lost, and that a stop then left cut short, is put again whole.

use std::{
    io::{self, Cursor},
    sync::{Arc, Mutex},
};

use anima_hub::{builds, events::process_log::subscriber};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::TimeDelta;
use serde_json::Value;
use sqlx::PgPool;

use super::{
    fake_r2::FakeR2,
    setup::{
        KEY_PASSWORD, LINUX_FEED, Publishing, public_key, ready, releases, secret_files,
        uploaded_linux,
    },
};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Opens the bucket of `releases` with this thread's log in a buffer; whether it opened, and
/// the log.
fn opened_with_log(releases: &anima_hub::config::Releases) -> (bool, String) {
    let capture = Capture::default();
    let writer = capture.clone();
    let log = subscriber("trace", move || writer.clone()).unwrap();
    let guard = tracing::subscriber::set_default(log);
    tracing::callsite::rebuild_interest_cache();
    let opened = builds::open_bucket(releases).unwrap().is_some();
    tracing::error!("capture control line");
    drop(guard);
    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains("capture control line"),
        "the log was not captured"
    );
    (opened, text)
}

#[tokio::test]
async fn r_116_a_wrong_password_or_another_public_key_turns_publishing_off_and_logs_no_key() {
    let fake = FakeR2::start().await;
    let (folder, pair) = secret_files();
    let key_file = std::fs::read_to_string(folder.path().join("updater_key")).unwrap();
    let good = releases(&fake, folder.path(), Some(public_key(&pair)));
    assert!(opened_with_log(&good).0);

    std::fs::write(&good.updater_key_password_file, "not the password").unwrap();
    let (opened, wrong) = opened_with_log(&good);
    assert!(!opened);
    assert!(wrong.contains("publishing is off"), "{wrong}");
    assert!(
        wrong.contains("the password does not open the key file"),
        "{wrong}"
    );

    std::fs::write(&good.updater_key_password_file, KEY_PASSWORD).unwrap();
    let other = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let mismatched = releases(&fake, folder.path(), Some(public_key(&other)));
    let (opened, mismatch) = opened_with_log(&mismatched);
    assert!(!opened);
    assert!(
        mismatch.contains("the updater key is not the key of ANIMA_UPDATER_PUBLIC_KEY"),
        "{mismatch}"
    );

    let unset = releases(&fake, folder.path(), None);
    assert!(!opened_with_log(&unset).0);

    let key_line = String::from_utf8(
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key_file.trim())
            .unwrap(),
    )
    .unwrap();
    let key_line = key_line.lines().nth(1).unwrap().to_owned();
    for log in [wrong, mismatch] {
        for secret in [
            key_file.trim(),
            key_line.as_str(),
            KEY_PASSWORD,
            "not the password",
        ] {
            assert!(!log.contains(secret), "the log holds key text: {log}");
        }
    }
}

/// D-44, CR-26: a key made with no password, which `tauri signer generate` still seals with
/// scrypt, opens with an empty password file. A missing key or password file, or an empty
/// password for a key made with one, turns publishing off with a fixed reason; the bucket is not
/// opened, and the hub goes on.
#[tokio::test]
async fn d_44_cr_26_an_empty_password_opens_only_a_key_made_without_one() {
    let fake = FakeR2::start().await;
    let (bare_folder, _) = secret_files();
    let bare = minisign::KeyPair::generate_encrypted_keypair(Some(String::new())).unwrap();
    let sealed = bare.sk.to_box(None).unwrap().into_string();
    // `EdScB2`: scrypt-sealed, with the empty password.
    assert!(sealed.contains("\nRWRTY0Iy"), "the key is not sealed");
    std::fs::write(
        bare_folder.path().join("updater_key"),
        STANDARD.encode(sealed),
    )
    .unwrap();
    std::fs::write(bare_folder.path().join("updater_key_password"), "").unwrap();
    let bare_settings = releases(&fake, bare_folder.path(), Some(public_key(&bare)));
    assert!(opened_with_log(&bare_settings).0);

    let (folder, pair) = secret_files();
    let settings = releases(&fake, folder.path(), Some(public_key(&pair)));

    std::fs::write(&settings.updater_key_password_file, "").unwrap();
    let (opened, empty) = opened_with_log(&settings);
    assert!(!opened);
    assert!(
        empty.contains("the key file is encrypted and needs its password"),
        "{empty}"
    );

    std::fs::remove_file(&settings.updater_key_password_file).unwrap();
    let (opened, no_password) = opened_with_log(&settings);
    assert!(!opened);
    assert!(
        no_password.contains("the updater key password file cannot be read"),
        "{no_password}"
    );

    std::fs::write(&settings.updater_key_password_file, KEY_PASSWORD).unwrap();
    std::fs::remove_file(&settings.updater_key_file).unwrap();
    let (opened, no_key) = opened_with_log(&settings);
    assert!(!opened);
    assert!(
        no_key.contains("the updater key file cannot be read"),
        "{no_key}"
    );
    for log in [empty, no_password, no_key] {
        assert!(
            !log.contains(KEY_PASSWORD),
            "the log holds the password: {log}"
        );
    }
}

/// D-25: the `.sig` write lands but its answer is lost, so the try waits; before the next try
/// the object is cut short, as a stop mid-write would leave it. The next try signs the same
/// bytes again, puts the `.sig` whole and publishes a feed whose signature is that `.sig`.
#[sqlx::test(migrations = "./migrations")]
async fn d_25_r_116_a_retry_after_a_partial_sig_write_puts_the_same_sig_whole(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, files) = uploaded_linux(&publishing, "0.1.12").await;
    let sig = format!("linux-x86_64/0.1.12/{}.sig", files[0].name);
    publishing.fake.with(|store| store.lose = Some(sig.clone()));
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "uploading");
    let first = publishing.fake.object(&sig).unwrap().bytes;
    publishing.fake.with(|store| {
        store.lose = None;
        let cut = store.objects.get_mut(&sig).unwrap();
        cut.bytes.truncate(first.len() / 2);
    });
    assert_eq!(publishing.sweep(ready() + TimeDelta::minutes(2)).await, 1);
    assert_eq!(publishing.job(&held).await, ("published".to_owned(), None));
    assert_eq!(publishing.fake.object(&sig).unwrap().bytes, first);
    publishing.assert_signed("0.1.12", &files[0]);
}

/// D-39, C-1: a sent feed is confirmed only with a `.sig` by the hub's key. A `.sig` and a
/// `latest.json` that agree but are signed by another key, as a bucket writer could leave them,
/// fail the job instead of publishing it.
#[sqlx::test(migrations = "./migrations")]
async fn d_39_a_sent_feed_signed_by_another_key_is_not_confirmed(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, files) = uploaded_linux(&publishing, "0.1.12").await;
    publishing.fake.with(|store| {
        store.lose = Some(LINUX_FEED.to_owned());
        store.tamper = Some(LINUX_FEED.to_owned());
    });
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "uploading");
    let other = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let comment = "timestamp:1888000000\tfile:Anima\tversion:0.1.12";
    let bytes = Cursor::new(&files[0].bytes);
    let signed = minisign::sign(Some(&other.pk), &other.sk, bytes, Some(comment), None).unwrap();
    let foreign = STANDARD.encode(signed.into_string());
    let sig = format!("linux-x86_64/0.1.12/{}.sig", files[0].name);
    publishing.fake.with(|store| {
        store.lose = None;
        store.tamper = None;
        let feed = store.objects.get_mut(LINUX_FEED).unwrap();
        let mut json: Value = serde_json::from_slice(&feed.bytes).unwrap();
        json["platforms"]["linux-x86_64"]["signature"] = Value::String(foreign.clone());
        feed.bytes = json.to_string().into_bytes();
        store.objects.get_mut(&sig).unwrap().bytes = foreign.clone().into_bytes();
    });
    assert_eq!(publishing.sweep(ready() + TimeDelta::minutes(2)).await, 1);
    let (state, reason) = publishing.job(&held).await;
    assert_eq!(state, "failed");
    assert!(reason.unwrap().contains("does not verify"));
}
