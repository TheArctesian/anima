//! Port 80: the ACME challenge and the redirect, never content (SF-NEW-routeheaders).

use std::collections::HashMap;

use crate::{door_header, plain::answer};

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn a_known_name_is_sent_to_https_and_an_unknown_one_gets_nothing() {
    let names = vec!["photos.example.com".to_owned()];
    let out = text(&answer(
        "Photos.Example.com",
        "/album?id=1",
        &names,
        &HashMap::new(),
    ));
    assert!(out.starts_with("HTTP/1.1 308"));
    assert!(out.contains("location: https://photos.example.com/album?id=1\r\n"));
    let other = text(&answer("hub.anima.ing", "/", &names, &HashMap::new()));
    assert!(other.starts_with("HTTP/1.1 404"));
}

#[test]
fn an_open_challenge_is_answered_and_a_different_token_is_not() {
    let challenges = HashMap::from([("tok".to_owned(), "tok.thumb".to_owned())]);
    let out = text(&answer(
        "photos.example.com",
        "/.well-known/acme-challenge/tok",
        &[],
        &challenges,
    ));
    assert!(out.starts_with("HTTP/1.1 200") && out.ends_with("\r\n\r\ntok.thumb"));
    let other = text(&answer(
        "photos.example.com",
        "/.well-known/acme-challenge/x",
        &[],
        &challenges,
    ));
    assert!(other.starts_with("HTTP/1.1 404"));
}

#[test]
fn the_proxy_header_gives_the_client_only_in_its_own_form() {
    let line = "PROXY TCP4 198.51.100.7 203.0.113.1 50000 443";
    assert_eq!(
        door_header::client(line),
        Some("198.51.100.7".parse().unwrap())
    );
    assert_eq!(door_header::client("PROXY TCP6 198.51.100.7 x 1 2"), None);
    assert_eq!(door_header::client("GET / HTTP/1.1"), None);
}

#[test]
fn only_an_address_of_the_door_may_send_a_proxy_header() {
    let door: std::net::IpAddr = "10.0.0.5".parse().unwrap();
    let client: std::net::IpAddr = "198.51.100.7".parse().unwrap();
    assert!(door_header::from_door(door, &[door]));
    assert!(door_header::from_door(
        "::ffff:10.0.0.5".parse().unwrap(),
        &[door]
    ));
    assert!(
        !door_header::from_door(client, &[door]),
        "a client cannot claim the address of a different client"
    );
    assert!(
        !door_header::from_door(door, &[]),
        "with no address of the door, no peer is the door"
    );
}
