//! The feed check (BLD-065, BLD-067 to BLD-069, UPD-009).

use serde_json::{Value, json};

use super::file;
use crate::builds::{
    model::{Target, UploadedFile},
    publishing::feed::{self, CheckedFeed, Expected, feed_folders},
    version::Version,
};

const BASE: &str = "https://releases.anima.ing";
const APPIMAGE: &str = "Anima_0.1.12_amd64.AppImage";
const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ=";

fn linux_files() -> Vec<UploadedFile> {
    vec![file(APPIMAGE, b"app"), file("SHA256SUMS", b"sums")]
}

fn version(text: &str) -> Version {
    text.parse().unwrap()
}

fn expected<'a>(target: Target, files: &'a [UploadedFile], floor: Option<&str>) -> Expected<'a> {
    Expected {
        target,
        version: version("0.1.12"),
        public_base: BASE,
        files,
        floor: floor.map(version),
    }
}

fn entry(url: &str) -> Value {
    json!({ "signature": SIGNATURE, "url": url })
}

fn linux_feed(platforms: &Value) -> String {
    json!({
        "version": "0.1.12",
        "notes": "Anima 0.1.12",
        "pub_date": "2030-05-05T12:00:00Z",
        "platforms": platforms,
    })
    .to_string()
}

fn appimage_url() -> String {
    format!("{BASE}/linux-x86_64/0.1.12/{APPIMAGE}")
}

fn check_linux(feed: &str, floor: Option<&str>) -> Result<CheckedFeed, String> {
    let files = linux_files();
    feed::check(feed, &expected(Target::LinuxX86_64, &files, floor))
}

#[test]
fn bld_065_a_feed_of_the_job_names_its_updater_file_and_signature() {
    let feed = linux_feed(&json!({ "linux-x86_64": entry(&appimage_url()) }));
    let checked = check_linux(&feed, None).unwrap();
    assert_eq!(checked.updater, APPIMAGE);
    assert_eq!(checked.signature, SIGNATURE);
}

#[test]
fn bld_065_a_feed_with_a_foreign_or_extra_platform_key_is_refused() {
    let url = appimage_url();
    for platforms in [
        json!({ "darwin-aarch64": entry(&url) }),
        json!({ "linux-x86_64": entry(&url), "windows-x86_64": entry(&url) }),
        json!({}),
    ] {
        let refused = check_linux(&linux_feed(&platforms), None).unwrap_err();
        assert_eq!(
            refused,
            "The feed of linux-x86_64 must list exactly linux-x86_64."
        );
    }
}

#[test]
fn bld_065_a_url_on_another_host_or_folder_is_refused() {
    for url in [
        format!("https://evil.example/linux-x86_64/0.1.12/{APPIMAGE}"),
        format!("https://releases.anima.ing.evil.example/linux-x86_64/0.1.12/{APPIMAGE}"),
        format!("http://releases.anima.ing/linux-x86_64/0.1.12/{APPIMAGE}"),
        format!("{BASE}/linux-x86_64/0.1.11/{APPIMAGE}"),
        format!("{BASE}/darwin-universal/0.1.12/{APPIMAGE}"),
        format!("{BASE}/linux-x86_64/0.1.12/../0.1.12/{APPIMAGE}"),
        format!("{BASE}/linux-x86_64/0.1.12/Other.AppImage"),
    ] {
        let feed = linux_feed(&json!({ "linux-x86_64": entry(&url) }));
        let refused = check_linux(&feed, None).unwrap_err();
        assert!(
            refused.starts_with("Each feed entry needs"),
            "{url}: {refused}"
        );
    }
}

#[test]
fn bld_065_the_feed_must_name_the_updater_file_not_the_sums() {
    let url = format!("{BASE}/linux-x86_64/0.1.12/SHA256SUMS");
    let feed = linux_feed(&json!({ "linux-x86_64": entry(&url) }));
    assert_eq!(
        check_linux(&feed, None).unwrap_err(),
        "The feed does not name the updater file of linux-x86_64."
    );
}

#[test]
fn bld_065_another_version_an_extra_field_or_a_bad_entry_is_refused() {
    let platforms = json!({ "linux-x86_64": entry(&appimage_url()) });
    let mut other_version: Value = serde_json::from_str(&linux_feed(&platforms)).unwrap();
    other_version["version"] = json!("0.1.13");
    assert_eq!(
        check_linux(&other_version.to_string(), None).unwrap_err(),
        "The feed is not for version 0.1.12."
    );
    let mut extra: Value = serde_json::from_str(&linux_feed(&platforms)).unwrap();
    extra["url"] = json!("https://evil.example");
    assert!(check_linux(&extra.to_string(), None).is_err());
    let with_extra_entry_field = json!({
        "linux-x86_64": { "signature": SIGNATURE, "url": appimage_url(), "with_elevated_task": true }
    });
    assert!(check_linux(&linux_feed(&with_extra_entry_field), None).is_err());
    let empty_signature = json!({ "linux-x86_64": { "signature": " ", "url": appimage_url() } });
    assert!(check_linux(&linux_feed(&empty_signature), None).is_err());
    assert!(check_linux("[]", None).is_err());
}

#[test]
fn bld_069_a_feed_below_the_published_version_is_refused() {
    let feed = linux_feed(&json!({ "linux-x86_64": entry(&appimage_url()) }));
    assert_eq!(
        check_linux(&feed, Some("0.1.13")).unwrap_err(),
        "linux-x86_64 has published 0.1.13, so a feed of 0.1.12 would go back."
    );
    assert!(check_linux(&feed, Some("0.1.12")).is_ok());
    assert!(check_linux(&feed, Some("0.1.11")).is_ok());
}

#[test]
fn bld_067_a_universal_feed_lists_both_mac_keys_with_one_entry() {
    let files = vec![
        file("Anima.app.tar.gz", b"app"),
        file("Anima_0.1.12_universal.dmg", b"dmg"),
        file("SHA256SUMS", b"sums"),
    ];
    let url = format!("{BASE}/darwin-universal/0.1.12/Anima.app.tar.gz");
    let both = linux_feed(&json!({ "darwin-aarch64": entry(&url), "darwin-x86_64": entry(&url) }));
    let darwin = expected(Target::DarwinUniversal, &files, None);
    assert_eq!(
        feed::check(&both, &darwin).unwrap().updater,
        "Anima.app.tar.gz"
    );
    let one = linux_feed(&json!({ "darwin-aarch64": entry(&url) }));
    assert!(feed::check(&one, &darwin).is_err());
    let dmg_url = format!("{BASE}/darwin-universal/0.1.12/Anima_0.1.12_universal.dmg");
    let split =
        linux_feed(&json!({ "darwin-aarch64": entry(&url), "darwin-x86_64": entry(&dmg_url) }));
    assert!(feed::check(&split, &darwin).is_err());
}

#[test]
fn bld_067_bld_068_feed_folders_follow_the_target_and_developer_id() {
    assert_eq!(feed_folders(Target::LinuxX86_64, false), ["linux-x86_64"]);
    assert_eq!(
        feed_folders(Target::DarwinUniversal, false),
        [] as [&str; 0]
    );
    assert_eq!(
        feed_folders(Target::DarwinUniversal, true),
        ["darwin-aarch64", "darwin-x86_64"]
    );
}
