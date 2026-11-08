//! The rules of an `uploaded` report (BLD-063, D-5) and of the release cursor (API-006).

use chrono::{TimeDelta, Utc};

use super::{RELEASE, start};
use crate::builds::{
    model::{Commit, Release, UploadedFile},
    page, upload,
    version::Version,
};

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn file(name: &str, sha256: &str, size: i64) -> UploadedFile {
    UploadedFile {
        name: name.to_owned(),
        sha256: sha256.to_owned(),
        size,
    }
}

#[test]
fn bld_063_the_files_are_plain_names_with_a_sha256_and_a_size() {
    let good = [
        file("Anima_0.1.12_amd64.AppImage", &SHA.to_uppercase(), 120_000_000),
        file("SHA256SUMS", SHA, 200),
    ];
    let kept = upload::files(&good).unwrap();
    assert_eq!(kept[0].sha256, SHA);
    for bad in [
        vec![],
        vec![file("../latest.json", SHA, 1)],
        vec![file(".hidden", SHA, 1)],
        vec![file("a/b", SHA, 1)],
        vec![file("SHA256SUMS", SHA, 1), file("SHA256SUMS", SHA, 1)],
        vec![file("x", &SHA[..63], 1)],
        vec![file("x", SHA, 0)],
        vec![file("x", SHA, 1); 9],
    ] {
        assert_eq!(upload::files(&bad).unwrap_err().field, "files", "{bad:?}");
    }
}

#[test]
fn bld_063_the_proposed_feed_is_a_json_object_of_at_most_64_kib() {
    assert!(upload::feed(r#"{"version":"0.1.12","platforms":{}}"#).is_ok());
    for bad in ["", "[]", "\"x\"", "{", &format!("{{\"a\":\"{}\"}}", "x".repeat(65_536))] {
        assert_eq!(upload::feed(bad).unwrap_err().field, "feed");
    }
}

fn release(at: chrono::DateTime<Utc>) -> Release {
    Release {
        id: RELEASE.parse().unwrap(),
        version: Version::new(0, 1, 12),
        commit: Commit("a".repeat(40)),
        created_by: "operator".to_owned(),
        created_at: at,
        jobs: Vec::new(),
    }
}

#[test]
fn api_006_a_release_cursor_reads_back_only_in_its_exact_form() {
    let release = release(start() + TimeDelta::microseconds(7));
    let cursor = page::cursor(&release);
    assert_eq!(
        page::position(&cursor).unwrap(),
        (release.created_at, release.id)
    );
    let hyphenated = format!("r1.{}.{}", release.created_at.timestamp_micros(), RELEASE);
    for bad in [
        String::new(),
        "r1.".to_owned(),
        hyphenated,
        cursor.replace("r1.", "e1."),
        format!("{cursor}0"),
    ] {
        assert_eq!(page::position(&bad).unwrap_err().field, "after", "{bad}");
    }
}

#[test]
fn api_006_a_page_holds_1_to_100_releases() {
    assert!(page::page_size(0).is_err());
    assert!(page::page_size(101).is_err());
    assert_eq!(page::page_size(100), Ok(100));
}
