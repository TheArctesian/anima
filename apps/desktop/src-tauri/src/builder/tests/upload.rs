//! The upload of a job's files (BLD-060, BLD-063, D-22): only the presigned URL's own headers to
//! an `https` address for the job's own object, only the bytes the bundle record names, the
//! shell's own `SHA256SUMS`, and no redirect followed.

use std::{fs, path::Path};

use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpListener,
};

use super::COMMIT;
use crate::builder::{
    artifact,
    bundled::Bundle,
    error::BuilderError,
    job::{Commit, Target, Version},
    upload::{
        ReleaseFile, Upload, UploadHeader, bound_updater, checked, send, sendable, sha256_of, sums,
    },
};

fn upload(url: &str, headers: &[(&str, &str)]) -> Upload {
    Upload {
        name: "Anima_0.1.12_amd64.AppImage".to_owned(),
        url: url.to_owned(),
        headers: headers
            .iter()
            .map(|(name, value)| UploadHeader {
                name: (*name).to_owned(),
                value: (*value).to_owned(),
            })
            .collect(),
    }
}

/// The job's object the test uploads.
const OBJECT: &str = "linux-x86_64/0.1.12/Anima_0.1.12_amd64.AppImage";

const FIXED: [(&str, &str); 3] = [
    ("content-length", "5"),
    ("cache-control", "public, max-age=3600"),
    ("content-type", "application/vnd.appimage"),
];

#[test]
fn bld_060_an_upload_goes_to_https_or_this_computer_with_its_own_headers() {
    for url in [
        format!(
            "https://account.r2.cloudflarestorage.com/anima-releases/{OBJECT}?X-Amz-Signature=ab"
        ),
        format!("http://127.0.0.1:9000/anima-releases/{OBJECT}"),
        format!("http://[::1]:9000/anima-releases/{OBJECT}"),
    ] {
        let put = checked(&upload(&url, &FIXED), 5, OBJECT).unwrap();
        assert_eq!(put.cache_control, "public, max-age=3600");
        assert_eq!(
            put.content_type.as_deref(),
            Some("application/vnd.appimage")
        );
    }
}

#[test]
fn bld_060_any_other_address_header_or_length_is_refused() {
    for url in [
        format!("http://account.r2.cloudflarestorage.com/anima-releases/{OBJECT}"),
        format!("https://user:pass@account.r2.cloudflarestorage.com/anima-releases/{OBJECT}"),
        "file:///etc/passwd".to_owned(),
        "not a url".to_owned(),
    ] {
        assert_eq!(
            checked(&upload(&url, &FIXED), 5, OBJECT).unwrap_err(),
            BuilderError::Upload,
            "{url}"
        );
    }
    let https = format!("https://account.r2.cloudflarestorage.com/anima-releases/{OBJECT}");
    let refused: [&[(&str, &str)]; 5] = [
        &[FIXED[0], FIXED[1], ("x-amz-acl", "public-read")],
        &[FIXED[0], FIXED[1], ("Cache-Control", "no-cache")],
        &[FIXED[0], FIXED[1], FIXED[2], ("content-type", "text/html")],
        &[("content-length", "6"), FIXED[1]],
        &[FIXED[0]],
    ];
    for headers in refused {
        assert!(
            checked(&upload(&https, headers), 5, OBJECT).is_err(),
            "{headers:?}"
        );
    }
}

/// S-5: a URL the hub gave for another object, of another job or another version, is refused,
/// so the bytes of this file never reach it.
#[test]
fn bld_060_an_upload_goes_only_to_the_jobs_own_object() {
    let base = "https://account.r2.cloudflarestorage.com/anima-releases";
    for path in [
        "linux-x86_64/latest.json",
        "Anima-x86_64.AppImage",
        "linux-x86_64/0.1.11/Anima_0.1.12_amd64.AppImage",
        "darwin-universal/0.1.12/Anima_0.1.12_amd64.AppImage",
        "linux-x86_64/0.1.12/SHA256SUMS",
        "x/linux-x86_64/0.1.12/Anima_0.1.12_amd64.AppImagex",
    ] {
        let url = format!("{base}/{path}?X-Amz-Signature=ab");
        assert!(checked(&upload(&url, &FIXED), 5, OBJECT).is_err(), "{path}");
    }
}

/// S-5: `http` to this computer is for a test or a debug build only.
#[test]
fn bld_060_a_release_build_sends_over_https_only() {
    let loopback = reqwest::Url::parse("http://127.0.0.1:9000/anima-releases/x").unwrap();
    assert!(sendable(&loopback, true));
    assert!(!sendable(&loopback, false));
    let https = reqwest::Url::parse("https://account.r2.cloudflarestorage.com/x").unwrap();
    assert!(sendable(&https, false));
}

/// UPD-010: the shell writes `SHA256SUMS` from the bundle's own files, one line each, by name.
#[test]
fn upd_010_the_sums_list_each_file_by_name() {
    let file = |name: &str, digit: &str| ReleaseFile {
        name: name.to_owned(),
        sha256: digit.repeat(64),
        size: 1,
    };
    let files = [
        file("Anima_0.1.12_universal.dmg", "b"),
        file("Anima.app.tar.gz", "a"),
    ];
    assert_eq!(
        sums(&files),
        format!(
            "{}  Anima.app.tar.gz\n{}  Anima_0.1.12_universal.dmg\n",
            "a".repeat(64),
            "b".repeat(64)
        )
    );
}

fn bundle_of(clone: &Path, bytes: &[u8]) -> Bundle {
    let version = Version::parse("0.1.12").unwrap();
    let path = artifact::updater_file(clone, Target::LinuxX86_64, &version);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    Bundle {
        commit: Commit::parse(COMMIT).unwrap(),
        target: Target::LinuxX86_64,
        version,
        path,
        sha256: sha256_of(bytes).unwrap(),
    }
}

#[test]
fn d_22_only_the_bytes_the_bundle_recorded_are_uploaded() {
    let clone = tempfile::tempdir().unwrap();
    let bundle = bundle_of(clone.path(), b"an AppImage");
    assert_eq!(
        bound_updater(clone.path(), &bundle).unwrap(),
        b"an AppImage"
    );
    fs::write(&bundle.path, b"another AppImage").unwrap();
    assert_eq!(
        bound_updater(clone.path(), &bundle).unwrap_err(),
        BuilderError::Bundle
    );
}

#[test]
fn bld_063_the_dmg_is_a_macos_file_only() {
    let clone = Path::new("/data/builds/anima");
    let version = Version::parse("0.1.12").unwrap();
    assert_eq!(
        artifact::dmg_file(clone, Target::DarwinUniversal, &version).unwrap(),
        Path::new(
            "/data/builds/anima/target/universal-apple-darwin/release/bundle/dmg/Anima_0.1.12_universal.dmg"
        )
    );
    assert_eq!(
        artifact::dmg_file(clone, Target::LinuxX86_64, &version),
        None
    );
}

/// A one-request HTTP server on this computer that answers `status`, sending `location` along;
/// returns its URL and the request it got.
async fn one_request(
    status: &'static str,
    location: String,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/anima-releases/{OBJECT}",
        listener.local_addr().unwrap()
    );
    let served = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        while !String::from_utf8_lossy(&request).contains("hello") {
            let read = stream.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let answer =
            format!("HTTP/1.1 {status}\r\nlocation: {location}\r\ncontent-length: 0\r\n\r\n");
        stream.write_all(answer.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&request).into_owned()
    });
    (url, served)
}

#[tokio::test]
async fn bld_060_the_put_carries_the_fixed_headers_and_follows_no_redirect() {
    let (url, served) = one_request("200 OK", String::new()).await;
    let put = checked(&upload(&url, &FIXED), 5, OBJECT).unwrap();
    send(put, b"hello".to_vec()).await.unwrap();
    let request = served.await.unwrap().to_ascii_lowercase();
    let line = format!("put /anima-releases/{} ", OBJECT.to_ascii_lowercase());
    assert!(request.starts_with(&line), "{request}");
    assert!(request.contains("cache-control: public, max-age=3600"));
    assert!(request.contains("content-type: application/vnd.appimage"));
    assert!(request.contains("content-length: 5"));
    // The redirect leads to a server that would take the file: only not following it fails.
    let (elsewhere, _) = one_request("200 OK", String::new()).await;
    let (url, _) = one_request("307 Temporary Redirect", elsewhere).await;
    let put = checked(&upload(&url, &FIXED), 5, OBJECT).unwrap();
    assert_eq!(
        send(put, b"hello".to_vec()).await.unwrap_err(),
        BuilderError::Upload
    );
}
