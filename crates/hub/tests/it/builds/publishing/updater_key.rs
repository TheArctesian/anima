//! The hub's updater key at start and its `.sig` on a retry (R-116, SEC-004, LOG-007, D-25): a
//! key that does not open with its password, or opens to another public key, turns publishing
//! off with a fixed warning that names no key text, and the hub still has its other parts; a
//! `.sig` write whose answer was lost, and that a stop then left cut short, is put again whole.

use std::{
    io,
    sync::{Arc, Mutex},
};

use anima_hub::{builds, events::process_log::subscriber};
use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    fake_r2::FakeR2,
    setup::{KEY_PASSWORD, Publishing, public_key, ready, releases, secret_files, uploaded_linux},
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

/// D-44, CR-26: a missing key or password file, or an empty password for an encrypted key,
/// turns publishing off with a fixed reason; the bucket is not opened, and the hub goes on.
#[tokio::test]
async fn d_44_cr_26_a_missing_file_or_an_empty_password_for_an_encrypted_key_turns_publishing_off()
{
    let fake = FakeR2::start().await;
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
