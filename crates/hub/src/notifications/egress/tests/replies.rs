//! How a reply is read: one deadline for the whole send, a status that decides at once, and at
//! most 4 KiB of the body (D-17, S-B, C-1, C-2).

use std::{net::SocketAddr, time::Duration};

use tokio::time::Instant;

use super::{
    http::{post, url},
    local,
    servers::{Behaviour, certificate, serve},
};
use crate::notifications::egress::{
    SendError, TIME_LIMIT,
    http::{REPLY_CAP, discard_reply},
};

/// How late past the time limit a send may end on a loaded machine.
const SLACK: Duration = Duration::from_secs(3);

/// Asserts that a send that waited for the time limit took it, and no second limit after it
/// (C-1, C-2).
fn ended_at_the_limit(started: Instant) {
    let took = started.elapsed();
    assert!(
        took >= TIME_LIMIT.saturating_sub(Duration::from_millis(100)),
        "{took:?}"
    );
    assert!(took < TIME_LIMIT + SLACK, "{took:?}");
}

#[tokio::test]
async fn d_17_a_server_that_never_answers_ends_at_the_time_limit() {
    let server = serve(Behaviour::Silent).await;
    let started = Instant::now();

    let sent = post(&local(), &format!("https://localhost:{}/", server.port)).await;

    assert_eq!(sent, Err(SendError::TimedOut));
    ended_at_the_limit(started);
    server.settle().await;
    assert_eq!(server.connections(), 1);
}

const OK_HEAD: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n";
const UNAVAILABLE_HEAD: &[u8] = b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 100\r\n\r\n";

#[tokio::test]
async fn d_17_a_2xx_whose_body_stalls_is_sent_and_ends_at_the_one_time_limit() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::Stalled(OK_HEAD, Duration::ZERO, acceptor)).await;
    let started = Instant::now();

    let sent = post(
        &local().trusting(der),
        &format!("https://localhost:{}/", server.port),
    )
    .await;

    // The status decided (S-B): a stalled body is not a time-out, and its read ends at the
    // send's own deadline, with no fresh limit of its own (C-2).
    assert_eq!(sent, Ok(()));
    ended_at_the_limit(started);
    server.settle().await;
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn d_17_a_late_2xx_head_leaves_the_body_read_only_what_is_left_of_the_limit() {
    let (der, acceptor) = certificate("localhost");
    let late = TIME_LIMIT / 2;
    let server = serve(Behaviour::Stalled(OK_HEAD, late, acceptor)).await;
    let started = Instant::now();

    let sent = post(
        &local().trusting(der),
        &format!("https://localhost:{}/", server.port),
    )
    .await;

    // The head came halfway through the limit; a fresh limit for the body read would end the
    // send at one and a half limits (C-2).
    assert_eq!(sent, Ok(()));
    ended_at_the_limit(started);
}

#[tokio::test]
async fn d_17_a_failing_status_decides_at_once_though_its_body_stalls() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::Stalled(
        UNAVAILABLE_HEAD,
        Duration::ZERO,
        acceptor,
    ))
    .await;
    let started = Instant::now();

    let sent = post(
        &local().trusting(der),
        &format!("https://localhost:{}/", server.port),
    )
    .await;

    assert_eq!(sent, Err(SendError::HttpStatus(503)));
    assert!(
        started.elapsed() < TIME_LIMIT / 2,
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn d_17_an_endless_reply_is_read_only_to_the_cap() {
    let server = serve(Behaviour::Endless(None)).await;
    let target: SocketAddr = ([127, 0, 0, 1], server.port).into();
    let client = local().pinned_client(Some("localhost"), &[target]).unwrap();
    let mut response = client
        .post(url(&format!("http://localhost:{}/", server.port)))
        .send()
        .await
        .unwrap();

    let read = discard_reply(&mut response).await;

    // The body never ends, so the read stopped at the cap, within the one chunk that
    // crossed it.
    assert!(read >= REPLY_CAP, "read {read}");
    assert!(read < 1024 * 1024, "read {read}");
}

#[tokio::test]
async fn d_17_an_endless_reply_ends_the_send_and_closes_the_connection() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::Endless(Some(acceptor))).await;

    let sent = post(
        &local().trusting(der),
        &format!("https://localhost:{}/", server.port),
    )
    .await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    // Read to the time limit, loopback would carry gigabytes; the socket buffers hold a few
    // megabytes once the client stops reading and closes.
    assert!(server.written() < 64 * 1024 * 1024, "{}", server.written());
}
