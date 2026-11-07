//! Each `refused` reason (K4 Section 9): the transfer flags read at each request, a missing or
//! refused path, and an upload that would replace a file, each with its audit entry.

use std::sync::atomic::Ordering;

use serde_json::json;
use sqlx::PgPool;

use super::bench::bench;

fn refused(reason: &str) -> serde_json::Value {
    json!({ "type": "refused", "reason": reason })
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_the_flags_are_read_at_each_request(pool: PgPool) {
    let bench = bench(&pool).await;
    bench.server.put("a.txt", b"abc");
    let mut client = bench.open().await;
    assert_eq!(client.download("/a.txt").await, b"abc");
    bench.flags(true, false).await;
    client.send(r#"{"type":"download","path":"/a.txt"}"#).await;
    assert_eq!(client.text().await, refused("DOWNLOAD_OFF"));
    client
        .send(r#"{"type":"upload","path":"/b.txt","size":0}"#)
        .await;
    assert_eq!(client.text().await, json!({ "type": "done" }));
    bench.flags(false, true).await;
    client
        .send(r#"{"type":"upload","path":"/c.txt","size":2}"#)
        .await;
    client.send_binary(b"hi").await;
    assert_eq!(client.text().await, refused("UPLOAD_OFF"));
    assert_eq!(bench.server.read("c.txt"), None);
    assert_eq!(client.download("/a.txt").await, b"abc");
    let audits = bench.audits().await;
    let seen: Vec<(String, serde_json::Value, serde_json::Value)> = audits
        .into_iter()
        .map(|(action, _, _, change)| (action, change["outcome"].clone(), change["reason"].clone()))
        .collect();
    assert_eq!(
        seen,
        [
            ("download".to_owned(), json!("done"), json!(null)),
            (
                "download".to_owned(),
                json!("refused"),
                json!("DOWNLOAD_OFF")
            ),
            ("upload".to_owned(), json!("done"), json!(null)),
            ("upload".to_owned(), json!("refused"), json!("UPLOAD_OFF")),
            ("download".to_owned(), json!("done"), json!(null)),
        ]
    );
    let events = bench.events().await;
    assert!(events.iter().any(|(_, _, _, message)| message
        == "The hub refused a file download from the device."));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_missing_or_refused_path_is_refused(pool: PgPool) {
    let bench = bench(&pool).await;
    std::fs::create_dir(bench.server.root.path().join("dir")).unwrap();
    let mut client = bench.open().await;
    for (request, reason) in [
        (r#"{"type":"download","path":"/missing"}"#, "NOT_FOUND"),
        (r#"{"type":"download","path":"/dir"}"#, "NOT_FOUND"),
        (r#"{"type":"list","path":"/missing"}"#, "NOT_FOUND"),
        (r#"{"type":"download","path":"/denied/a"}"#, "DENIED"),
        (r#"{"type":"list","path":"/denied"}"#, "DENIED"),
        (r#"{"type":"upload","path":"/denied/a","size":0}"#, "DENIED"),
        (
            r#"{"type":"upload","path":"/nowhere/a","size":0}"#,
            "NOT_FOUND",
        ),
    ] {
        client.send(request).await;
        assert_eq!(client.text().await, refused(reason), "{request}");
    }
    let audits = bench.audits().await;
    assert_eq!(audits.len(), 5, "lists are not audited");
    let (_, _, _, change) = &audits[0];
    assert_eq!(change["path"], "/missing");
    assert_eq!(change["reason"], "NOT_FOUND");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_an_upload_never_replaces_a_file(pool: PgPool) {
    let bench = bench(&pool).await;
    bench.server.put("kept.txt", b"original");
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/kept.txt","size":3}"#)
        .await;
    client.send_binary(b"new").await;
    assert_eq!(client.text().await, refused("EXISTS"));
    assert_eq!(bench.server.read("kept.txt").unwrap(), b"original");
    // The refused upload's bytes were dropped, so the next request is read as one.
    client.send(r#"{"type":"list","path":"/"}"#).await;
    assert_eq!(client.text().await["type"], "entries");
    let (action, _, _, change) = bench.audits().await.remove(0);
    assert_eq!(
        (action.as_str(), change["reason"].clone()),
        ("upload", json!("EXISTS"))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_the_device_refuses_a_replace_the_hub_did_not_see(pool: PgPool) {
    let bench = bench(&pool).await;
    bench.server.put("kept.txt", b"original");
    bench.server.quirks.hide_files.store(true, Ordering::SeqCst);
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/kept.txt","size":3}"#)
        .await;
    client.send_binary(b"new").await;
    assert_eq!(client.text().await, refused("DENIED"));
    assert_eq!(bench.server.read("kept.txt").unwrap(), b"original");
}
