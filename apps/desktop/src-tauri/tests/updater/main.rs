//! UPD-002, BLD-067, BLD-068, BLD-069, DNS-007: the real `tauri-plugin-updater`, configured from
//! the shipped `plugins.updater` block of `tauri.conf.json`, against a feed served over loopback.
//! It proves that the app asks for `<target>-<arch>/latest.json`, takes the newer build, installs
//! nothing whose signature fails, and keeps what it has when the feed has no entry for its target.
//!
//! Nothing here leaves 127.0.0.1, and no key outlives its test: each test makes a throwaway
//! minisign key pair in memory, and its public half replaces the placeholder `pubkey` of the shipped
//! configuration, so the operator's real key is never needed.

mod feed;

use feed::{Channel, Listing, Served};

/// The version the mock app reports.
const APP_VERSION: &str = "0.1.0";

#[tokio::test(flavor = "multi_thread")]
async fn bld_067_the_app_asks_for_the_feed_of_its_own_target_and_arch() {
    let channel = Channel::new();
    let build = channel.build("0.2.0");
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    let update = served.check().await.unwrap().expect("a newer build is offered");
    assert_eq!(update.version, "0.2.0");
    assert_eq!(served.requested(), [feed::feed_path()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn upd_002_the_newer_build_is_taken_and_its_bytes_verify() {
    let channel = Channel::new();
    let build = channel.build("0.2.0");
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    let update = served.check().await.unwrap().expect("0.2.0 is newer than 0.1.0");
    let bytes = update.download(|_, _| {}, || {}).await.unwrap();
    assert_eq!(bytes, build.bytes());
}

#[tokio::test(flavor = "multi_thread")]
async fn upd_002_the_current_version_is_not_offered_again() {
    let channel = Channel::new();
    let build = channel.build(APP_VERSION);
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    assert!(served.check().await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_069_an_older_signed_build_is_installed_as_a_rollback() {
    let channel = Channel::new();
    let build = channel.build("0.0.9");
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    let update = served.check().await.unwrap().expect("allowDowngrades takes a rollback");
    let bytes = update.download(|_, _| {}, || {}).await.unwrap();
    assert_eq!(bytes, build.bytes());
}

#[tokio::test(flavor = "multi_thread")]
async fn upd_002_a_signature_by_another_key_is_refused() {
    let channel = Channel::new();
    let build = Channel::new().build("0.2.0");
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    let update = served.check().await.unwrap().expect("the feed names a newer build");
    let refused = update.download(|_, _| {}, || {}).await.unwrap_err();
    assert!(refused.to_string().contains("different key"), "{refused}");
}

#[tokio::test(flavor = "multi_thread")]
async fn upd_002_bytes_changed_after_signing_are_refused() {
    let channel = Channel::new();
    let build = channel.build_tampered("0.2.0");
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    let update = served.check().await.unwrap().expect("the feed names a newer build");
    let refused = update.download(|_, _| {}, || {}).await.unwrap_err();
    assert!(refused.to_string().contains("signature verification failed"), "{refused}");
}

#[tokio::test(flavor = "multi_thread")]
async fn upd_002_a_build_signed_for_another_version_is_refused() {
    let channel = Channel::new();
    // `requireSignedVersion` (shipped true) stops a feed that labels an old build as a new one.
    let build = channel.build_signed_for("0.9.0", "0.2.0");
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    let update = served.check().await.unwrap().expect("the feed names a newer build");
    let refused = update.download(|_, _| {}, || {}).await.unwrap_err();
    assert!(refused.to_string().contains("signed for version"), "{refused}");
}

#[tokio::test(flavor = "multi_thread")]
async fn upd_002_a_signature_that_names_no_version_is_refused() {
    let channel = Channel::new();
    let build = channel.build_unversioned("0.2.0");
    let served = Served::start(&channel, Some(Listing::here(&build))).await;
    let update = served.check().await.unwrap().expect("the feed names a newer build");
    let refused = update.download(|_, _| {}, || {}).await.unwrap_err();
    assert!(refused.to_string().contains("does not specify the version"), "{refused}");
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_068_a_target_with_no_feed_is_a_missing_release() {
    let served = Served::start(&Channel::new(), None).await;
    let refused = served.check().await.map(|_| ()).unwrap_err();
    assert!(refused.to_string().contains("valid release JSON"), "{refused}");
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_068_a_feed_with_no_entry_for_this_target_installs_nothing() {
    let channel = Channel::new();
    let build = channel.build("0.2.0");
    let served = Served::start(&channel, Some(Listing::elsewhere(&build))).await;
    let refused = served.check().await.map(|_| ()).unwrap_err();
    assert!(refused.to_string().contains("fallback platforms"), "{refused}");
}

#[test]
fn dns_007_the_shipped_endpoint_is_the_releases_origin_with_target_and_arch() {
    assert_eq!(
        feed::shipped_endpoint(),
        "https://releases.anima.ing/{{target}}-{{arch}}/latest.json"
    );
}
