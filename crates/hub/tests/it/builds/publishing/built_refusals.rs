//! What the release commands of the workflow refuse, against an in-process R2 (R-179;
//! BLD-002, BLD-046, BLD-064, BLD-069): after each refusal the feed is as it was.

use sha2::{Digest as _, Sha256};

use super::{
    built::{Workflow, folder_of},
    fake_r2::{Object, hex},
    setup::{File, LINUX_FEED, LINUX_STABLE, linux_files},
};

#[tokio::test(flavor = "multi_thread")]
async fn bld_069_a_version_below_the_published_one_is_refused_and_writes_nothing() {
    let workflow = Workflow::start().await;
    let newer = folder_of(&linux_files("0.2.2"));
    let first = workflow
        .release(("linux-x86_64", "0.2.2"), newer.path())
        .await;
    assert!(first.passed, "{}", first.printed);
    let before = workflow.writes();
    let older = folder_of(&linux_files("0.2.1"));

    let refused = workflow
        .release(("linux-x86_64", "0.2.1"), older.path())
        .await;

    assert!(!refused.passed);
    assert!(
        refused.printed.contains("would go back"),
        "{}",
        refused.printed
    );
    assert_eq!(workflow.writes(), before);
    assert_eq!(workflow.feed(LINUX_FEED)["version"], "0.2.2");
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_002_a_published_version_is_not_published_again_with_other_bytes() {
    let workflow = Workflow::start().await;
    let folder = folder_of(&linux_files("0.2.2"));
    let first = workflow
        .release(("linux-x86_64", "0.2.2"), folder.path())
        .await;
    assert!(first.passed, "{}", first.printed);
    let feed = workflow.bytes(LINUX_FEED);

    let again = workflow
        .run("publish", ("linux-x86_64", "0.2.2"), folder.path())
        .await;
    assert!(again.passed, "the same release again: {}", again.printed);
    assert_eq!(workflow.bytes(LINUX_FEED), feed);

    let app = File::new("Anima_0.2.2_amd64.AppImage", "another build of 0.2.2");
    let sums = format!("{}  {}\n", hex(&Sha256::digest(&app.bytes)), app.name);
    let other = folder_of(&[app, File::new("SHA256SUMS", sums)]);
    let before = workflow.writes();
    let refused = workflow
        .release(("linux-x86_64", "0.2.2"), other.path())
        .await;

    assert!(!refused.passed);
    assert_eq!(workflow.writes(), before);
    assert_eq!(workflow.bytes(LINUX_FEED), feed);
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_002_a_version_folder_that_holds_other_bytes_is_refused_before_any_feed() {
    let workflow = Workflow::start().await;
    let folder = folder_of(&linux_files("0.2.2"));
    let held = "linux-x86_64/0.2.2/Anima_0.2.2_amd64.AppImage";
    workflow.fake.with(|store| {
        let object = Object::new("bytes of an earlier build");
        store.objects.insert(held.to_owned(), object);
    });

    let refused = workflow
        .release(("linux-x86_64", "0.2.2"), folder.path())
        .await;

    assert!(!refused.passed);
    assert!(
        refused.printed.contains("with other bytes"),
        "{}",
        refused.printed
    );
    assert!(workflow.bytes(LINUX_FEED).is_none());
    assert!(workflow.bytes(LINUX_STABLE).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_064_a_file_that_is_not_the_one_its_build_job_reported_is_refused() {
    let workflow = Workflow::start().await;
    let files = linux_files("0.2.2");
    let folder = folder_of(&files);
    let signed = workflow
        .run("sign", ("linux-x86_64", "0.2.2"), folder.path())
        .await;
    assert!(signed.passed, "{}", signed.printed);
    // SHA256SUMS still holds the digest the build job reported; the file and its signature
    // are those of another step.
    std::fs::write(
        folder.path().join(&files[0].name),
        "changed after the build",
    )
    .unwrap();
    let signed = workflow
        .run("sign", ("linux-x86_64", "0.2.2"), folder.path())
        .await;
    assert!(signed.passed, "{}", signed.printed);

    let refused = workflow
        .run("publish", ("linux-x86_64", "0.2.2"), folder.path())
        .await;

    assert!(!refused.passed);
    assert!(
        refused.printed.contains("SHA256SUMS"),
        "{}",
        refused.printed
    );
    assert_eq!(workflow.writes(), Vec::<String>::new());
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_046_a_signature_of_another_key_or_another_version_is_refused() {
    let workflow = Workflow::start().await;
    let other = Workflow::start().await;
    let folder = folder_of(&linux_files("0.2.2"));
    let signed = other
        .run("sign", ("linux-x86_64", "0.2.2"), folder.path())
        .await;
    assert!(signed.passed, "{}", signed.printed);

    let refused = workflow
        .run("publish", ("linux-x86_64", "0.2.2"), folder.path())
        .await;

    assert!(!refused.passed);
    assert!(
        refused.printed.contains("does not verify"),
        "{}",
        refused.printed
    );
    assert_eq!(workflow.writes(), Vec::<String>::new());

    // The right key, but the signature names 0.2.2 and the release says 0.2.3.
    let files = linux_files("0.2.2");
    let renamed = File::new("Anima_0.2.3_amd64.AppImage", files[0].bytes.clone());
    let sums = format!(
        "{}  {}\n",
        hex(&Sha256::digest(&renamed.bytes)),
        renamed.name
    );
    let next = folder_of(&[renamed, File::new("SHA256SUMS", sums)]);
    let signed = workflow
        .run("sign", ("linux-x86_64", "0.2.2"), folder.path())
        .await;
    assert!(signed.passed, "{}", signed.printed);
    std::fs::copy(
        folder.path().join("Anima_0.2.2_amd64.AppImage.sig"),
        next.path().join("Anima_0.2.3_amd64.AppImage.sig"),
    )
    .unwrap();
    let refused = workflow
        .run("publish", ("linux-x86_64", "0.2.3"), next.path())
        .await;
    assert!(!refused.passed);
    assert_eq!(workflow.writes(), Vec::<String>::new());
}

#[tokio::test(flavor = "multi_thread")]
async fn bld_064_a_file_the_bucket_gives_back_changed_stops_the_release_before_the_feed() {
    let workflow = Workflow::start().await;
    let folder = folder_of(&linux_files("0.2.2"));
    let versioned = "linux-x86_64/0.2.2/Anima_0.2.2_amd64.AppImage";
    workflow
        .fake
        .with(|store| store.tamper = Some(versioned.to_owned()));

    let refused = workflow
        .release(("linux-x86_64", "0.2.2"), folder.path())
        .await;

    assert!(!refused.passed);
    assert!(workflow.bytes(LINUX_FEED).is_none());
    assert!(workflow.bytes(LINUX_STABLE).is_none());
}
