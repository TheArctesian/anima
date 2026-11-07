//! Listing, downloading and uploading over the files tunnel (K4 Section 9), each with what it
//! leaves behind: one audit entry with the path and size and no content, and one event; and
//! nothing of either in the process log (LOG-007).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    bench::{Bench, bench, bench_with},
    socket::{self, Received},
};
use crate::{
    events::process_log,
    sessions::tests::fixture::{Capture, stored_text},
};

/// Bytes no stored text may hold: the file content of these tests.
fn content(len: usize) -> Vec<u8> {
    (0..len).map(|i| b"QZXJ"[i % 4]).collect()
}

/// The one audit entry so far: action, operator, client and change.
async fn only_audit(bench: &Bench) -> (String, Value) {
    let audits = bench.audits().await;
    assert_eq!(audits.len(), 1, "{audits:?}");
    let (action, operator, client, change) = audits.into_iter().next().unwrap();
    assert_eq!(
        (operator.as_str(), client.as_str()),
        ("operator", "127.0.0.1")
    );
    (action, change)
}

/// The stored audit entries and events hold no file content (LOG-007).
async fn no_content_stored(bench: &Bench) {
    let stored = stored_text(&bench.db).await;
    assert!(!stored.contains("QZXJ"), "{stored}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_directory_lists_its_files_folders_and_links(pool: PgPool) {
    let bench = bench(&pool).await;
    bench.server.put("a.txt", b"hello");
    std::fs::create_dir(bench.server.root.path().join("sub")).unwrap();
    std::os::unix::fs::symlink("a.txt", bench.server.root.path().join("to-a")).unwrap();
    let mut client = bench.open().await;
    client.send(r#"{"type":"list","path":"/"}"#).await;
    let listed = client.text().await;
    assert_eq!(listed["type"], "entries");
    assert_eq!(listed["path"], "/");
    let entries = listed["entries"].as_array().unwrap();
    let shown: Vec<(&str, &str)> = entries
        .iter()
        .map(|entry| {
            (
                entry["name"].as_str().unwrap(),
                entry["kind"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(shown, [("a.txt", "file"), ("sub", "dir"), ("to-a", "link")]);
    assert_eq!(entries[0]["size"], 5);
    assert!(entries[0]["modifiedAt"].is_string());
    assert_eq!(entries[0].as_object().unwrap().len(), 4);
    assert_eq!(bench.audits().await, [], "a listing is not a transfer");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_listing_stops_at_its_cap(pool: PgPool) {
    let bench = bench_with(&pool, |files| files.max_entries = 2).await;
    for name in ["a", "b", "c"] {
        bench.server.put(name, b"x");
    }
    let mut client = bench.open().await;
    client.send(r#"{"type":"list","path":"/"}"#).await;
    assert_eq!(client.text().await["entries"].as_array().unwrap().len(), 2);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_download_sends_its_size_its_bytes_and_done(pool: PgPool) {
    let bench = bench(&pool).await;
    let bytes = content(100_000);
    bench.server.put("big.bin", &bytes);
    let mut client = bench.open().await;
    assert_eq!(client.download("/big.bin").await, bytes);
    let (action, change) = only_audit(&bench).await;
    assert_eq!(action, "download");
    assert_eq!(change["path"], "/big.bin");
    assert_eq!(
        (change["size"].clone(), change["bytes"].clone()),
        (json!(100_000), json!(100_000))
    );
    assert_eq!(change["outcome"], "done");
    assert_eq!(change["sessionId"], bench.session.to_string());
    let events = bench.events().await;
    let (severity, source, device, message) = events.last().unwrap();
    assert_eq!((severity.as_str(), source.as_str()), ("info", "session"));
    assert_eq!(device.as_deref(), Some("docker-host"));
    assert_eq!(message, "An operator downloaded a file from the device.");
    no_content_stored(&bench).await;
    // The tunnel takes the next request.
    bench.server.put("empty", b"");
    assert_eq!(client.download("/empty").await, b"");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_an_upload_of_exactly_its_size_lands_on_the_device(pool: PgPool) {
    let bench = bench(&pool).await;
    let bytes = content(70_000);
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/new.bin","size":70000}"#)
        .await;
    client.send_binary(&bytes[..65_536]).await;
    client.send_binary(&bytes[65_536..]).await;
    assert_eq!(client.text().await, json!({ "type": "done" }));
    assert_eq!(bench.server.read("new.bin"), Some(bytes));
    let (action, change) = only_audit(&bench).await;
    assert_eq!(action, "upload");
    assert_eq!(change["path"], "/new.bin");
    assert_eq!(
        (change["size"].clone(), change["outcome"].clone()),
        (json!(70_000), json!("done"))
    );
    no_content_stored(&bench).await;
    client
        .send(r#"{"type":"upload","path":"/zero","size":0}"#)
        .await;
    assert_eq!(client.text().await, json!({ "type": "done" }));
    assert_eq!(bench.server.read("zero"), Some(Vec::new()));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_an_upload_opens_its_file_exclusively(pool: PgPool) {
    let bench = bench(&pool).await;
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/x","size":1}"#)
        .await;
    client.send_binary(b"x").await;
    assert_eq!(client.text().await, json!({ "type": "done" }));
    // SFTP v3: WRITE 0x02, CREATE 0x08, EXCLUDE 0x20; never TRUNCATE (0x10) or APPEND.
    assert_eq!(bench.server.opens(), [("/x".to_owned(), 0x2A)]);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_an_upload_past_its_size_is_a_protocol_error(pool: PgPool) {
    let bench = bench(&pool).await;
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/over","size":4}"#)
        .await;
    client.send_binary(b"abc").await;
    client.send_binary(b"de").await;
    assert_eq!(client.close_code().await, Some(1002));
    assert_eq!(
        bench.server.read("over"),
        None,
        "the unfinished upload is removed"
    );
    let (_, change) = only_audit(&bench).await;
    assert_eq!(
        (change["bytes"].clone(), change["outcome"].clone()),
        (json!(3), json!("cancelled"))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_cancelled_download_ends_with_done(pool: PgPool) {
    let bench = bench(&pool).await;
    bench.server.put("big.bin", &content(1024 * 1024));
    bench
        .server
        .quirks
        .slow_reads
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"download","path":"/big.bin"}"#)
        .await;
    assert_eq!(client.text().await["size"], 1024 * 1024);
    assert!(matches!(client.next().await, Received::Binary(_)));
    client.send(r#"{"type":"cancel"}"#).await;
    loop {
        match client.next().await {
            Received::Binary(_) => {}
            Received::Text(text) => {
                assert_eq!(text, json!({ "type": "done" }));
                break;
            }
            closed @ Received::Closed(_) => panic!("{closed:?}"),
        }
    }
    let (_, change) = only_audit(&bench).await;
    assert_eq!(change["outcome"], "cancelled");
    assert!(change["bytes"].as_u64().unwrap() < 1024 * 1024);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_cancelled_upload_is_removed(pool: PgPool) {
    let bench = bench(&pool).await;
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/half","size":100}"#)
        .await;
    client.send_binary(&content(50)).await;
    client.send(r#"{"type":"cancel"}"#).await;
    assert_eq!(client.text().await, json!({ "type": "done" }));
    assert_eq!(bench.server.read("half"), None);
    let (_, change) = only_audit(&bench).await;
    assert_eq!(
        (change["bytes"].clone(), change["outcome"].clone()),
        (json!(50), json!("cancelled"))
    );
    // A cancel with nothing in flight is ignored.
    client.send(r#"{"type":"cancel"}"#).await;
    client.send(r#"{"type":"list","path":"/"}"#).await;
    assert_eq!(client.text().await["type"], "entries");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn log_007_sec_009_no_path_content_or_token_reaches_the_process_log(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in the Guacamole tunnel's test: a second registered dispatcher makes every callsite
    // registration ask all of them, so another test thread cannot disable these callsites.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    tracing::info!("log_007 capture marker");

    let bench = bench(&pool).await;
    let bytes = content(70_000);
    bench.server.put("kept-private.bin", &bytes);
    let token = bench.token().await;
    let mut client = socket::open(&bench.url(&token)).await;
    assert_eq!(client.download("/kept-private.bin").await, bytes);
    client
        .send(r#"{"type":"upload","path":"/sent-private.bin","size":70000}"#)
        .await;
    client.send_binary(&bytes[..65_536]).await;
    client.send_binary(&bytes[65_536..]).await;
    assert_eq!(client.text().await, json!({ "type": "done" }));
    assert_eq!(bench.audits().await.len(), 2, "both transfers ran");

    let log = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        log.contains("log_007 capture marker"),
        "the capture works: {log}"
    );
    for secret in [
        "kept-private",
        "sent-private",
        "QZXJ",
        token.as_str(),
        "token=",
    ] {
        assert!(!log.contains(secret), "{secret} in the log");
    }
}
