//! The certificate reload (R-021 H-2): an hourly look by time and file state, a swap when the
//! files change, and the old certificate kept when the new files do not load.

use std::{sync::Arc, time::Duration};

use chrono::TimeDelta;

use super::super::{
    certificate::{Current, Fault},
    reload::{CHECK_EVERY, Outcome, Reload, Served, Stamp, Stamps, check, load, stamps},
};
use super::{Files, now, provider, throwaway};

fn stamp(len: u64) -> Stamps {
    let file = Stamp {
        modified: None,
        len,
    };
    Stamps {
        cert: file,
        key: file,
    }
}

#[test]
fn r_021_a_look_is_due_one_hour_after_the_last() {
    let mut reload = Reload::new(stamp(1), now());
    assert!(!reload.begin(now() + CHECK_EVERY - TimeDelta::seconds(1)));
    assert!(reload.begin(now() + CHECK_EVERY));
    assert!(!reload.begin(now() + CHECK_EVERY + TimeDelta::minutes(59)));
    assert!(reload.begin(now() + CHECK_EVERY * 2));
}

#[test]
fn r_021_the_wait_runs_to_the_next_look_and_never_past_an_hour() {
    let reload = Reload::new(stamp(1), now());
    assert_eq!(reload.wait(now()), Duration::from_hours(1));
    assert_eq!(
        reload.wait(now() + TimeDelta::minutes(45)),
        Duration::from_mins(15)
    );
    assert_eq!(reload.wait(now() + CHECK_EVERY * 3), Duration::ZERO);
    assert_eq!(
        reload.wait(now() - TimeDelta::days(2)),
        Duration::from_hours(1),
        "a clock that went back"
    );
}

#[test]
fn r_021_only_a_change_in_either_file_asks_for_a_load() {
    let mut reload = Reload::new(stamp(1), now());
    assert!(!reload.changed(stamp(1)));
    assert!(reload.changed(stamp(2)));
    let key_only = Stamps {
        key: Stamp {
            modified: None,
            len: 9,
        },
        ..stamp(1)
    };
    assert!(reload.changed(key_only));
    reload.loaded(stamp(2));
    assert!(!reload.changed(stamp(2)));
    assert!(reload.changed(stamp(1)));
}

/// Served from `files` as `listen` sets it up at `now()`.
async fn served(files: &Files) -> (Reload, Served) {
    let provider = provider();
    let key = load(&files.paths, &provider).await.unwrap();
    let reload = Reload::new(stamps(&files.paths).unwrap(), now());
    let served = Served {
        files: files.paths.clone(),
        current: Arc::new(Current::new(key)),
        provider,
    };
    (reload, served)
}

fn serving(served: &Served) -> Vec<u8> {
    served.current.get().cert[0].to_vec()
}

#[tokio::test]
async fn r_021_h_2_changed_files_swap_the_certificate_at_the_next_look() {
    let files = Files::new();
    let (first, second) = (throwaway(), throwaway());
    files.write(&first.cert, &first.key, 0);
    let (mut reload, served) = served(&files).await;
    assert_eq!(serving(&served), first.der);

    files.write(&second.cert, &second.key, 60);
    let early = check(&mut reload, now() + TimeDelta::minutes(30), &served).await;
    assert_eq!(early, Outcome::NotDue);
    assert_eq!(serving(&served), first.der, "not before the hour");

    let due = check(&mut reload, now() + CHECK_EVERY, &served).await;
    assert_eq!(due, Outcome::Swapped);
    assert_eq!(serving(&served), second.der);

    let again = check(&mut reload, now() + CHECK_EVERY * 2, &served).await;
    assert_eq!(again, Outcome::Unchanged);
    assert_eq!(serving(&served), second.der);
}

#[tokio::test]
async fn h_2_the_single_tls_pem_reloads_and_a_mismatched_pair_keeps_the_old_certificate() {
    let files = Files::single();
    let (first, second) = (throwaway(), throwaway());
    let combined = |key: &str, cert: &str| format!("{key}{cert}");
    let write = |text: String, seconds| files.write(&text, &text, seconds);
    write(combined(&first.key, &first.cert), 0);
    let (mut reload, served) = served(&files).await;
    assert_eq!(serving(&served), first.der);

    write(combined(&first.key, &second.cert), 60);
    let mismatched = check(&mut reload, now() + CHECK_EVERY, &served).await;
    assert_eq!(mismatched, Outcome::Kept(Fault::KeyMismatch));
    assert_eq!(serving(&served), first.der);

    write(combined(&second.key, &second.cert), 120);
    let swapped = check(&mut reload, now() + CHECK_EVERY * 2, &served).await;
    assert_eq!(swapped, Outcome::Swapped);
    assert_eq!(serving(&served), second.der);
}

#[tokio::test]
async fn r_021_h_2_a_bad_new_certificate_keeps_the_old_one_and_is_tried_again() {
    let files = Files::new();
    let (good, other) = (throwaway(), throwaway());
    files.write(&good.cert, &good.key, 0);
    let (mut reload, served) = served(&files).await;

    files.write("not a certificate", &good.key, 60);
    let broken = check(&mut reload, now() + CHECK_EVERY, &served).await;
    assert_eq!(broken, Outcome::Kept(Fault::NoCertificate));
    assert_eq!(serving(&served), good.der);

    files.write(&other.cert, &good.key, 120);
    let mismatched = check(&mut reload, now() + CHECK_EVERY * 2, &served).await;
    assert_eq!(mismatched, Outcome::Kept(Fault::KeyMismatch));
    assert_eq!(serving(&served), good.der);

    std::fs::remove_file(&files.paths.cert).unwrap();
    let gone = check(&mut reload, now() + CHECK_EVERY * 3, &served).await;
    assert!(
        matches!(gone, Outcome::Kept(Fault::CertificateFile(_))),
        "{gone:?}"
    );
    assert_eq!(serving(&served), good.der);

    files.write(&other.cert, &other.key, 180);
    let fixed = check(&mut reload, now() + CHECK_EVERY * 4, &served).await;
    assert_eq!(fixed, Outcome::Swapped, "a failed load is retried");
    assert_eq!(serving(&served), other.der);
}
