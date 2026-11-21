//! One request and its answer over in-memory streams.

use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, duplex};

use crate::{
    head::Rewritten,
    relay::{relay, status},
};

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
