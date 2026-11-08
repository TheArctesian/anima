//! HTTP egress: checked address only, no second lookup, no redirect, TLS verified for the host
//! name, and at most 4 KiB of the reply read (P3-003, D-17).

use std::net::SocketAddr;

use reqwest::{
    Url,
    header::{HeaderMap, HeaderValue},
};

use super::{
    local, loopback_denied,
    servers::{Behaviour, certificate, serve},
};
use crate::{
    notifications::egress::{
        Egress, SendError,
        http::{REPLY_CAP, classify, discard_reply},
    },
    targets::{AddressPolicy, TargetError},
};

const NO_CONTENT: &[u8] = b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_static("Bearer synthetic"),
    );
    headers
}

async fn post(egress: &Egress, target: &str) -> Result<(), SendError> {
    egress
        .post(
            &url(target),
            headers(),
            b"{\"text\":\"web is down\"}".to_vec(),
        )
        .await
}

#[tokio::test]
async fn p3_003_a_host_name_is_reached_at_its_checked_address() {
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), None)).await;

    let sent = post(&local(), &format!("http://localhost:{}/hook", server.port)).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert_eq!(server.lines(), ["POST /hook HTTP/1.1"]);
}

#[tokio::test]
async fn p3_003_an_ip_literal_host_is_checked_and_reached() {
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), None)).await;

    let sent = post(&local(), &format!("http://127.0.0.1:{}/hook", server.port)).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.lines(), ["POST /hook HTTP/1.1"]);
}

#[tokio::test]
async fn p3_003_the_pinned_client_resolves_no_name_of_its_own() {
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), None)).await;
    let pinned: SocketAddr = ([127, 0, 0, 1], server.port).into();
    let client = local()
        .pinned_client(Some("pinned.invalid"), &[pinned])
        .unwrap();

    // `.invalid` never resolves (RFC 6761): the pinned name reaches the pinned address only
    // because the client looked nothing up.
    let pinned_reply = client
        .post(url(&format!("http://pinned.invalid:{}/", server.port)))
        .send()
        .await;
    let other = client
        .post(url(&format!("http://localhost:{}/", server.port)))
        .send()
        .await
        .unwrap_err();

    assert_eq!(pinned_reply.unwrap().status(), 204);
    assert_eq!(classify(&other), SendError::Target(TargetError::Unresolved));
    server.settle().await;
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn p3_003_a_refused_address_gets_no_connection() {
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), None)).await;
    let by_name = format!("http://localhost:{}/", server.port);
    let by_address = format!("http://127.0.0.1:{}/", server.port);
    let by_v6 = format!("http://[::ffff:127.0.0.1]:{}/", server.port);
    let default = Egress::new(AddressPolicy::default());

    let refused = Err(SendError::Target(TargetError::Refused));
    assert_eq!(post(&default, &by_name).await, refused);
    assert_eq!(post(&default, &by_address).await, refused);
    assert_eq!(post(&default, &by_v6).await, refused);
    assert_eq!(post(&loopback_denied(), &by_name).await, refused);
    assert_eq!(post(&loopback_denied(), &by_address).await, refused);
    server.settle().await;
    assert_eq!(server.connections(), 0);
}

#[tokio::test]
async fn p3_003_an_unresolved_name_or_another_scheme_is_never_sent() {
    let unresolved = post(&local(), "http://nothing-here.invalid/").await;
    let ftp = post(&local(), "ftp://localhost/").await;

    assert_eq!(unresolved, Err(SendError::Target(TargetError::Unresolved)));
    assert_eq!(ftp, Err(SendError::Url));
}

#[tokio::test]
async fn p3_003_a_redirect_is_a_failure_and_is_not_followed() {
    let second = serve(Behaviour::Http(NO_CONTENT.to_vec(), None)).await;
    let moved = format!(
        "HTTP/1.1 302 Found\r\nLocation: http://localhost:{}/\r\nContent-Length: 0\r\n\r\n",
        second.port
    );
    let first = serve(Behaviour::Http(moved.into_bytes(), None)).await;

    let sent = post(&local(), &format!("http://localhost:{}/", first.port)).await;

    assert_eq!(sent, Err(SendError::HttpStatus(302)));
    first.settle().await;
    second.settle().await;
    assert_eq!(first.connections(), 1);
    assert_eq!(second.connections(), 0);
}

#[tokio::test]
async fn d_17_an_untrusted_certificate_fails_before_the_request() {
    let (_, acceptor) = certificate("localhost");
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), Some(acceptor))).await;

    let sent = post(&local(), &format!("https://localhost:{}/", server.port)).await;

    assert_eq!(sent, Err(SendError::Tls));
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert_eq!(server.lines(), Vec::<String>::new());
}

#[tokio::test]
async fn d_17_a_certificate_for_another_name_fails_before_the_request() {
    let (der, acceptor) = certificate("other.test");
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), Some(acceptor))).await;

    let sent = post(
        &local().trusting(der),
        &format!("https://localhost:{}/", server.port),
    )
    .await;

    assert_eq!(sent, Err(SendError::Tls));
    server.settle().await;
    assert_eq!(server.lines(), Vec::<String>::new());
}

#[tokio::test]
async fn d_17_a_trusted_server_sees_the_host_name_as_sni() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), Some(acceptor))).await;

    let sent = post(
        &local().trusting(der),
        &format!("https://localhost:{}/hook", server.port),
    )
    .await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.server_names(), ["localhost"]);
    assert_eq!(server.lines(), ["POST /hook HTTP/1.1"]);
}

#[tokio::test]
async fn d_17_an_endless_reply_is_read_only_to_the_cap() {
    let server = serve(Behaviour::Endless).await;
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
    let server = serve(Behaviour::Endless).await;

    let sent = post(&local(), &format!("http://localhost:{}/", server.port)).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    // Read to the time limit, loopback would carry gigabytes; the socket buffers hold a few
    // megabytes once the client stops reading and closes.
    assert!(server.written() < 64 * 1024 * 1024, "{}", server.written());
}
