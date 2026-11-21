//! The head of a request (SF-NEW-routeheaders).

use std::net::IpAddr;

use crate::head::{Status, rewrite};

const CLIENT: &str = "198.51.100.7";

fn written(head: &str) -> Result<String, Status> {
    let client: IpAddr = CLIENT.parse().unwrap();
    rewrite(head.as_bytes(), "photos.example.com", client)
        .map(|rewritten| String::from_utf8(rewritten.head).unwrap())
}

#[test]
fn the_headers_of_the_client_and_the_hub_are_removed_and_set_again() {
    let head = "GET /a?b=c HTTP/1.1\r\nHost: photos.example.com\r\nForwarded: for=1.2.3.4\r\n\
                X-Forwarded-For: 1.2.3.4\r\nX-Forwarded-Proto: http\r\nX-Real-IP: 1.2.3.4\r\n\
                X-Anima-Account: someone\r\nAccept: text/html\r\n\r\n";
    let out = written(head).unwrap();
    for gone in [
        "Forwarded:",
        "X-Forwarded-For: 1.2.3.4",
        "X-Forwarded-Proto: http",
        "X-Real-IP",
        "X-Anima-Account",
    ] {
        assert!(!out.contains(gone), "{gone} in {out}");
    }
    assert!(out.contains(&format!("x-forwarded-for: {CLIENT}\r\n")));
    assert!(out.contains("x-forwarded-proto: https\r\n"));
    assert!(out.contains("Accept: text/html\r\n"));
    assert!(out.ends_with("connection: close\r\n\r\n"));
}

#[test]
fn the_cookies_of_a_hub_are_never_sent_and_the_others_are() {
    let head = "GET / HTTP/1.1\r\nHost: photos.example.com\r\n\
                Cookie: __Host-anima=secret; session=app; anima_ticket=x\r\n\r\n";
    let out = written(head).unwrap();
    assert!(out.contains("Cookie: session=app\r\n"));
    assert!(!out.contains("secret") && !out.contains("anima_ticket"));
    let only_hub =
        "GET / HTTP/1.1\r\nHost: photos.example.com\r\nCookie: __Host-anima=secret\r\n\r\n";
    assert!(!written(only_hub).unwrap().contains("Cookie"));
}

#[test]
fn a_host_that_is_not_the_sni_name_is_misdirected() {
    for host in ["hub.anima.ing", "photos.example.com.evil.test", ""] {
        let head = format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n");
        assert_eq!(written(&head), Err(Status::Misdirected), "{host}");
    }
    let no_host = "GET / HTTP/1.1\r\nAccept: */*\r\n\r\n";
    assert_eq!(written(no_host), Err(Status::Misdirected));
    let with_port = "GET / HTTP/1.1\r\nHost: Photos.Example.com:443\r\n\r\n";
    assert!(written(with_port).is_ok());
}

#[test]
fn a_smuggled_or_malformed_request_is_refused() {
    let chunked =
        "POST / HTTP/1.1\r\nHost: photos.example.com\r\nTransfer-Encoding: chunked\r\n\r\n";
    assert_eq!(written(chunked), Err(Status::LengthRequired));
    let two_hosts = "GET / HTTP/1.1\r\nHost: photos.example.com\r\nHost: hub.anima.ing\r\n\r\n";
    assert_eq!(written(two_hosts), Err(Status::BadRequest));
    let absolute = "GET http://hub.anima.ing/ HTTP/1.1\r\nHost: photos.example.com\r\n\r\n";
    assert_eq!(written(absolute), Err(Status::BadRequest));
    let folded = "GET / HTTP/1.1\r\nHost: photos.example.com\r\n X-Folded: yes\r\n\r\n";
    assert_eq!(written(folded), Err(Status::BadRequest));
    let http2 = "GET / HTTP/2.0\r\nHost: photos.example.com\r\n\r\n";
    assert_eq!(written(http2), Err(Status::NotImplemented));
}

#[test]
fn a_websocket_upgrade_keeps_its_connection_header() {
    let head = "GET /ws HTTP/1.1\r\nHost: photos.example.com\r\nConnection: Upgrade\r\n\
                Upgrade: websocket\r\n\r\n";
    let client: IpAddr = CLIENT.parse().unwrap();
    let rewritten = rewrite(head.as_bytes(), "photos.example.com", client).unwrap();
    assert!(rewritten.upgrade);
    assert!(
        String::from_utf8(rewritten.head)
            .unwrap()
            .ends_with("connection: upgrade\r\n\r\n")
    );
}
