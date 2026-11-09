//! HTTP egress: https only, checked address only, no second lookup, no redirect, TLS verified
//! for the host name, and at most 4 KiB of the reply read (P3-003, D-17, CR-14).

use std::{net::SocketAddr, time::Duration};

use reqwest::{
    Method, Url,
    header::{HeaderMap, HeaderValue},
};
use tokio::time::Instant;

use super::{
    local, loopback_denied,
    servers::{Behaviour, Server, certificate, serve},
};
use crate::{
    notifications::egress::{
        Egress, HttpRequest, SendError, TIME_LIMIT,
        http::{REPLY_CAP, classify, discard_reply},
    },
    targets::{AddressPolicy, TargetError},
};

const NO_CONTENT: &[u8] = b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";
const BODY: &[u8] = b"{\"text\":\"web is down\"}";

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_static("Bearer synthetic"),
    );
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers
}

fn request(method: Method, target: Url) -> HttpRequest {
    HttpRequest {
        method,
        url: target,
        headers: headers(),
        body: BODY.to_vec(),
    }
}

async fn post(egress: &Egress, target: &str) -> Result<(), SendError> {
    egress.send_http(request(Method::POST, url(target))).await
}

/// How late past the time limit a send may end on a loaded machine.
const SLACK: Duration = Duration::from_secs(3);

/// Asserts that a send that waited for the time limit took it, and no second limit after it
/// (C-1, C-2).
fn ended_at_the_limit(started: Instant) {
    let took = started.elapsed();
    assert!(took >= TIME_LIMIT.saturating_sub(Duration::from_millis(100)), "{took:?}");
    assert!(took < TIME_LIMIT + SLACK, "{took:?}");
}

/// A TLS server for `name` that answers `reply`, and an egress that trusts its certificate.
async fn tls_server(name: &str, reply: &[u8]) -> (Server, Egress) {
    let (der, acceptor) = certificate(name);
    let server = serve(Behaviour::Http(reply.to_vec(), Some(acceptor))).await;
    (server, local().trusting(der))
}

#[tokio::test]
async fn p3_003_a_host_name_is_reached_at_its_checked_address() {
    let (server, egress) = tls_server("localhost", NO_CONTENT).await;

    let sent = post(&egress, &format!("https://localhost:{}/hook", server.port)).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert_eq!(server.lines(), ["POST /hook HTTP/1.1"]);
}

#[tokio::test]
async fn p3_003_the_body_and_the_given_headers_are_sent() {
    let (server, egress) = tls_server("localhost", NO_CONTENT).await;

    let sent = post(&egress, &format!("https://localhost:{}/hook", server.port)).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.bodies(), [BODY]);
    let headers: Vec<String> = server
        .headers()
        .iter()
        .map(|line| line.to_ascii_lowercase())
        .collect();
    assert!(
        headers.contains(&"authorization: bearer synthetic".to_owned()),
        "{headers:?}"
    );
    assert!(
        headers.contains(&"content-type: application/json".to_owned()),
        "{headers:?}"
    );
}

#[tokio::test]
async fn p3_003_an_answer_outside_2xx_is_its_status() {
    for (reply, status) in [
        (
            &b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"[..],
            404,
        ),
        (
            &b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n"[..],
            503,
        ),
    ] {
        let (server, egress) = tls_server("localhost", reply).await;

        let sent = post(&egress, &format!("https://localhost:{}/", server.port)).await;

        assert_eq!(sent, Err(SendError::HttpStatus(status)));
    }
}

#[tokio::test]
async fn d_17_a_root_the_hub_cannot_read_is_a_setup_failure_with_no_connection() {
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), None)).await;

    let sent = post(
        &local().trusting(b"not a certificate".to_vec()),
        &format!("https://localhost:{}/", server.port),
    )
    .await;

    assert_eq!(sent, Err(SendError::Setup));
    server.settle().await;
    assert_eq!(server.connections(), 0);
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
    let server = serve(Behaviour::Stalled(OK_HEAD, acceptor)).await;
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
async fn d_17_a_failing_status_decides_at_once_though_its_body_stalls() {
    let (der, acceptor) = certificate("localhost");
    let server = serve(Behaviour::Stalled(UNAVAILABLE_HEAD, acceptor)).await;
    let started = Instant::now();

    let sent = post(
        &local().trusting(der),
        &format!("https://localhost:{}/", server.port),
    )
    .await;

    assert_eq!(sent, Err(SendError::HttpStatus(503)));
    assert!(started.elapsed() < TIME_LIMIT / 2, "{:?}", started.elapsed());
}

#[tokio::test]
async fn not_002_a_put_is_sent_as_a_put() {
    let (server, egress) = tls_server("localhost", NO_CONTENT).await;
    let target = url(&format!("https://localhost:{}/hook", server.port));

    let sent = egress.send_http(request(Method::PUT, target)).await;

    assert_eq!(sent, Ok(()));
    server.settle().await;
    assert_eq!(server.lines(), ["PUT /hook HTTP/1.1"]);
    assert_eq!(server.bodies(), [BODY]);
}

#[tokio::test]
async fn p3_003_an_ip_literal_host_is_checked_and_reached() {
    let (server, egress) = tls_server("127.0.0.1", NO_CONTENT).await;

    let sent = post(&egress, &format!("https://127.0.0.1:{}/hook", server.port)).await;

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
    let by_name = format!("https://localhost:{}/", server.port);
    let by_address = format!("https://127.0.0.1:{}/", server.port);
    let by_v6 = format!("https://[::ffff:127.0.0.1]:{}/", server.port);
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
    let unresolved = post(&local(), "https://nothing-here.invalid/").await;
    let ftp = post(&local(), "ftp://localhost/").await;

    assert_eq!(unresolved, Err(SendError::Target(TargetError::Unresolved)));
    assert_eq!(ftp, Err(SendError::Url));
}

#[tokio::test]
async fn cr_14_an_http_url_gets_no_lookup_and_no_connection() {
    let server = serve(Behaviour::Http(NO_CONTENT.to_vec(), None)).await;

    let by_name = post(&local(), &format!("http://localhost:{}/hook", server.port)).await;
    let by_address = post(&local(), &format!("http://127.0.0.1:{}/hook", server.port)).await;
    // `.invalid` never resolves: had the name been looked up, this would be `Unresolved`.
    let unresolvable = post(&local(), "http://nothing-here.invalid/").await;

    assert_eq!(by_name, Err(SendError::Url));
    assert_eq!(by_address, Err(SendError::Url));
    assert_eq!(unresolvable, Err(SendError::Url));
    server.settle().await;
    assert_eq!(server.connections(), 0);
}

#[tokio::test]
async fn p3_003_a_redirect_is_a_failure_and_is_not_followed() {
    let (second, _) = tls_server("localhost", NO_CONTENT).await;
    let moved = format!(
        "HTTP/1.1 302 Found\r\nLocation: https://localhost:{}/\r\nContent-Length: 0\r\n\r\n",
        second.port
    );
    let (first, egress) = tls_server("localhost", moved.as_bytes()).await;

    let sent = post(&egress, &format!("https://localhost:{}/", first.port)).await;

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
