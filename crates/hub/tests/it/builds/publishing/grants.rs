//! A builder gets upload URLs only for its target's release files, each signed with its content
//! type, and a job that fails leaves none of its files in the bucket (BLD-061, BLD-063, BLD-070).

use sqlx::PgPool;

use super::setup::{File, Held, LINUX, MACOS, Publishing, linux_files, ready, uploaded_linux};
use crate::{
    builds::harness::{data, enrol},
    hub::error,
};

/// The review's case: a page or a file of another target gets no URL, and nothing is granted.
#[sqlx::test(migrations = "./migrations")]
async fn bld_061_only_the_release_files_of_the_target_are_granted(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    publishing.release("0.1.12", "LINUX_X86_64").await;
    let held = publishing.uploading("pop-os", LINUX).await;
    let files = linux_files("0.1.12");
    for name in [
        "index.html",
        "Anima_0.1.12_universal.dmg",
        "Anima.app.tar.gz",
        "Anima_0.1.13_amd64.AppImage",
    ] {
        let other = File::new(name, "<script>alert(1)</script>");
        let response = publishing.grant(&held, &[files[1].clone(), other]).await;
        assert_eq!(error(&response).1, "VALIDATION", "{name}: {response}");
    }
    let granted: i64 = sqlx::query_scalar("SELECT count(*) FROM builds.job_files")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(granted, 0);
    let granted = data(publishing.grant(&held, &files).await);
    let kinds: Vec<(&str, &str)> = granted["buildUploadUrls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|upload| {
            let url = upload["url"].as_str().unwrap();
            assert!(url.contains("content-type"), "the content type is signed");
            let headers = upload["headers"].as_array().unwrap();
            let kind = headers
                .iter()
                .find(|h| h["name"] == "content-type")
                .unwrap();
            (
                upload["name"].as_str().unwrap(),
                kind["value"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("Anima_0.1.12_amd64.AppImage", "application/vnd.appimage"),
            ("SHA256SUMS", "text/plain; charset=utf-8"),
        ]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn bld_070_a_job_that_fails_leaves_none_of_its_files_in_the_bucket(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (held, files) = uploaded_linux(&publishing, "0.1.12").await;
    let keys: Vec<String> = files
        .iter()
        .map(|file| format!("linux-x86_64/0.1.12/{}", file.name))
        .collect();
    publishing
        .fake
        .with(|store| store.tamper = Some(keys[0].clone()));
    assert_eq!(publishing.sweep(ready()).await, 1);
    assert_eq!(publishing.job(&held).await.0, "failed");
    // The hub's `.sig` is deleted too, written or not; a delete of no object is a no-op (R-116).
    let mut keys = keys;
    keys.push(format!("linux-x86_64/0.1.12/{}.sig", files[0].name));
    assert_eq!(publishing.fake.with(|store| store.deletes.clone()), keys);
    for key in &keys {
        assert!(publishing.fake.object(key).is_none(), "{key}");
    }
}

/// A macOS job is granted only its archive, its DMG and `SHA256SUMS`.
#[sqlx::test(migrations = "./migrations")]
async fn bld_063_a_macos_job_is_granted_only_its_release_files(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    publishing.release("0.1.12", "DARWIN_UNIVERSAL").await;
    let held = publishing.uploading("mac-mini", MACOS).await;
    for name in [
        "index.html",
        "Anima_0.1.12_amd64.AppImage",
        "Anima_0.1.13_universal.dmg",
        "Anima.app.zip",
    ] {
        let other = File::new(name, "<script>alert(1)</script>");
        let response = publishing.grant(&held, &[other]).await;
        assert_eq!(error(&response).1, "VALIDATION", "{name}: {response}");
    }
    let files = [
        File::new("Anima.app.tar.gz", "archive"),
        File::new("Anima_0.1.12_universal.dmg", "disk image"),
        File::new("SHA256SUMS", "sums"),
    ];
    let granted = data(publishing.grant(&held, &files).await);
    assert_eq!(granted["buildUploadUrls"].as_array().unwrap().len(), 3);
}

/// The content types as stored: the bucket keeps what the URL signed.
#[sqlx::test(migrations = "./migrations")]
async fn bld_061_the_bucket_keeps_the_signed_content_type(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    let (_, files) = uploaded_linux(&publishing, "0.1.12").await;
    let key = format!("linux-x86_64/0.1.12/{}", files[0].name);
    let stored = publishing.fake.object(&key).unwrap().content_type;
    assert_eq!(stored.as_deref(), Some("application/vnd.appimage"));
}

/// R-108: a builder that does not hold the job's lease gets no upload URL for it.
#[sqlx::test(migrations = "./migrations")]
async fn r_108_only_the_lease_holder_gets_upload_urls(pool: PgPool) {
    let publishing = Publishing::start(&pool).await;
    publishing.release("0.1.12", "LINUX_X86_64").await;
    let held = publishing.uploading("pop-os", LINUX).await;
    let other = enrol(&publishing.hub, &publishing.operator, "fedora").await;
    let stranger = Held {
        job: held.job.clone(),
        token: other.token,
    };
    let response = publishing.grant(&stranger, &linux_files("0.1.12")).await;
    assert_eq!(error(&response).1, "FORBIDDEN", "{response}");
    let granted: i64 = sqlx::query_scalar("SELECT count(*) FROM builds.job_files")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(granted, 0);
}
