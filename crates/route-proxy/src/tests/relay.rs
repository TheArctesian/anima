//! One request and its answer over in-memory streams.

use std::time::Duration;

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, duplex};

use crate::{
    head::Rewritten,
    relay::{Idle, relay, status},
};

fn upgrade() -> Rewritten {
    Rewritten {
        head: b"GET /ws HTTP/1.1\r\nhost: a\r\nconnection: upgrade\r\n\r\n".to_vec(),
        body: 0,
        upgrade: true,
        target: "/ws".to_owned(),
    }
}

fn request(body: u64) -> Rewritten {
    Rewritten {
        head: b"POST / HTTP/1.1\r\nhost: a\r\nconnection: close\r\n\r\n".to_vec(),
        body,
        upgrade: false,
        target: "/".to_owned(),
    }
}

#[tokio::test]
async fn the_body_goes_as_its_length_says_and_no_byte_more() {
    let (mut client, mut client_side) = duplex(4096);
    let (mut app, mut app_side) = duplex(4096);
    client
        .write_all(b"lo, and a second request that must not pass")
        .await
        .unwrap();
    let length = request(5).head.len() + 5;
    let application = tokio::spawn(async move {
        let mut seen = vec![0_u8; length];
        app_side.read_exact(&mut seen).await.unwrap();
        app_side
            .write_all(b"HTTP/1.1 201 Created\r\n\r\nok")
            .await
            .unwrap();
        drop(app_side);
        seen
    });
    let relayed = relay(&mut client_side, b"hel", &mut app, &request(5))
        .await
        .unwrap();
    let seen = application.await.unwrap();
    assert!(seen.ends_with(b"\r\n\r\nhello"));
    assert_eq!(relayed.status, Some(201));
    assert_eq!(relayed.bytes_out, 26);
    drop(client_side);
    let mut answer = String::new();
    client.read_to_string(&mut answer).await.unwrap();
    assert_eq!(answer, "HTTP/1.1 201 Created\r\n\r\nok");
}

#[test]
fn the_status_is_read_only_from_a_status_line() {
    assert_eq!(status(b"HTTP/1.1 404 Not Found\r\n"), Some(404));
    assert_eq!(status(b"garbage"), None);
}

#[tokio::test]
async fn an_upgrade_goes_both_ways_only_after_a_101() {
    let (mut client, mut client_side) = duplex(4096);
    let (mut app, mut app_side) = duplex(4096);
    let length = upgrade().head.len();
    let application = tokio::spawn(async move {
        let mut head = vec![0_u8; length];
        app_side.read_exact(&mut head).await.unwrap();
        app_side
            .write_all(b"HTTP/1.1 101 Switching Protocols\r\n\r\n")
            .await
            .unwrap();
        let mut frame = [0_u8; 4];
        app_side.read_exact(&mut frame).await.unwrap();
        app_side.write_all(b"pong").await.unwrap();
        frame
    });
    let relaying = tokio::spawn(async move {
        relay(&mut client_side, b"", &mut app, &upgrade())
            .await
            .unwrap()
    });
    let mut answer = vec![0_u8; b"HTTP/1.1 101 Switching Protocols\r\n\r\n".len()];
    client.read_exact(&mut answer).await.unwrap();
    client.write_all(b"ping").await.unwrap();
    let mut pong = [0_u8; 4];
    client.read_exact(&mut pong).await.unwrap();
    assert_eq!(&pong, b"pong");
    assert_eq!(&application.await.unwrap(), b"ping");
    drop(client);
    assert_eq!(relaying.await.unwrap().status, Some(101));
}

#[tokio::test]
async fn an_upgrade_that_the_application_refuses_is_a_plain_answer() {
    let (mut client, mut client_side) = duplex(4096);
    let (mut app, mut app_side) = duplex(4096);
    let length = upgrade().head.len();
    let application = tokio::spawn(async move {
        let mut head = vec![0_u8; length];
        app_side.read_exact(&mut head).await.unwrap();
        app_side
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nno")
            .await
            .unwrap();
        app_side.shutdown().await.unwrap();
        let mut left = Vec::new();
        app_side.read_to_end(&mut left).await.unwrap();
        left
    });
    client
        .write_all(b"GET /secret HTTP/1.1\r\nhost: a\r\n\r\n")
        .await
        .unwrap();
    let relayed = relay(&mut client_side, b"", &mut app, &upgrade())
        .await
        .unwrap();
    assert_eq!(relayed.status, Some(200));
    drop(client_side);
    let mut answer = String::new();
    client.read_to_string(&mut answer).await.unwrap();
    assert!(answer.ends_with("\r\n\r\nno"));
    drop(app);
    let left = application.await.unwrap();
    assert!(
        left.is_empty(),
        "the bytes of the client never reach the application"
    );
}

#[tokio::test]
async fn a_stream_with_no_byte_for_its_idle_time_ends() {
    let (_quiet, side) = duplex(64);
    let mut idle = Idle::new(side, Duration::from_millis(50));
    let mut buffer = [0_u8; 8];
    let error = idle.read(&mut buffer).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
}

#[tokio::test]
async fn each_byte_starts_the_idle_time_again() {
    let (mut talking, side) = duplex(64);
    let mut idle = Idle::new(side, Duration::from_millis(200));
    let writer = tokio::spawn(async move {
        for _ in 0..4 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            talking.write_all(b"x").await.unwrap();
        }
    });
    let mut buffer = [0_u8; 1];
    for _ in 0..4 {
        idle.read_exact(&mut buffer).await.unwrap();
    }
    writer.await.unwrap();
}
