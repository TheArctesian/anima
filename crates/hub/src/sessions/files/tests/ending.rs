//! How the files tunnel ends (K4 Sections 8.5 and 9): with its SSH session, at shutdown, on a
//! client message outside Section 9, on client silence, and when the device stalls; and what
//! an upload cut short leaves behind.

use std::{sync::atomic::Ordering, time::Duration};

use sqlx::PgPool;

use super::{
    bench::{bench, bench_with},
    socket::Received,
};
use crate::sessions::{
    end::Actor,
    tests::fixture::{LOOPBACK, address, now, operator},
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_the_files_tunnel_ends_with_its_ssh_session(pool: PgPool) {
    let mut bench = bench(&pool).await;
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/part","size":10}"#)
        .await;
    client.send_binary(b"abcde").await;
    client.send(r#"{"type":"ping"}"#).await;
    // The upload is open on the device once the ping after its first frame was read.
    tokio::time::sleep(Duration::from_millis(200)).await;
    drop(bench.terminal.take());
    assert_eq!(client.close_code().await, Some(1000));
    assert_eq!(
        bench.server.read("part"),
        None,
        "the unfinished upload is removed"
    );
    let audits = bench.audits().await;
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].3["outcome"], "cancelled");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_end_session_ends_the_files_tunnel(pool: PgPool) {
    let bench = bench(&pool).await;
    let mut client = bench.open().await;
    let by = Actor {
        operator: operator(),
        client: address(LOOPBACK),
        at: now(),
    };
    let _ending = bench.live.end_by(bench.session, &by);
    assert_eq!(client.close_code().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_shutdown_ends_the_files_tunnel_with_1001(pool: PgPool) {
    let bench = bench(&pool).await;
    let mut client = bench.open().await;
    let _closing = bench.live.shut_down();
    assert_eq!(client.close_code().await, Some(1001));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_message_outside_section_9_closes_1002(pool: PgPool) {
    let bench = bench(&pool).await;
    let oversize = format!(r#"{{"type":"ping"}}{}"#, " ".repeat(4096));
    for text in [
        r#"{"type":"resize","cols":80,"rows":24}"#,
        oversize.as_str(),
    ] {
        let mut client = bench.open().await;
        client.send(text).await;
        assert_eq!(client.close_code().await, Some(1002), "{text:.40}");
    }
    let mut client = bench.open().await;
    client
        .send_unchecked_text(b"{\"type\":\"list\",\"path\":\"\xff\"}")
        .await;
    assert_eq!(client.close_code().await, Some(1002), "not UTF-8");
    let mut client = bench.open().await;
    client.send_binary(b"stray").await;
    assert_eq!(
        client.close_code().await,
        Some(1002),
        "binary without an upload"
    );
    let mut client = bench.open().await;
    client.send_binary(&vec![0; 64 * 1024 + 1]).await;
    assert_eq!(client.close_code().await, Some(1002), "over 64 KiB");
    let mut client = bench.open().await;
    client
        .send(r#"{"type":"upload","path":"/one","size":5}"#)
        .await;
    client.send(r#"{"type":"list","path":"/"}"#).await;
    assert_eq!(
        client.close_code().await,
        Some(1002),
        "one transfer at a time"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_client_silence_closes_4002_after_hub_pings(pool: PgPool) {
    let bench = bench_with(&pool, |files| {
        files.timing.hub_silence = Duration::from_millis(200);
        files.timing.client_idle = Duration::from_secs(1);
    })
    .await;
    let mut client = bench.open().await;
    assert_eq!(client.close_code().await, Some(4002));
    assert!(client.pings >= 2, "{} pings", client.pings);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_device_that_stops_answering_closes_1011(pool: PgPool) {
    let bench = bench_with(&pool, |files| files.timing.device = Duration::from_secs(1)).await;
    bench.server.put("a.txt", b"abc");
    bench
        .server
        .quirks
        .stall_reads
        .store(true, Ordering::SeqCst);
    let mut client = bench.open().await;
    client.send(r#"{"type":"download","path":"/a.txt"}"#).await;
    assert_eq!(client.text().await["size"], 3);
    assert_eq!(client.next().await, Received::Closed(Some(1011)));
    let audits = bench.audits().await;
    assert_eq!(audits[0].3["outcome"], "failed");
    let events = bench.events().await;
    let (severity, _, _, message) = events.last().unwrap();
    assert_eq!(
        (severity.as_str(), message.as_str()),
        ("warning", "A file download from the device failed.")
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_an_idle_tunnel_keeps_its_link_alive(pool: PgPool) {
    let bench = bench_with(&pool, |files| {
        files.timing.keep_alive = Duration::from_millis(100);
    })
    .await;
    let mut client = bench.open().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    client.send(r#"{"type":"list","path":"/"}"#).await;
    assert_eq!(client.text().await["type"], "entries");
    assert_eq!(bench.server.requests().len(), 2, "still the one channel");
}
