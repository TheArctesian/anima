//! The transport rules of K8 Section 1 (MET-020): no redirect, no proxy, no cookie.

use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

use anima_daemon::config::origin::Origin;
use anima_daemon::hub::answer::{Answer, Cause};
use anima_daemon::hub::link_socket::LinkSocketError;
use anima_daemon::hub::{HubClient, HubError, TIMEOUT};
use daemon_core::wire::{Announcements, Batch, RefusalCode};
use reqwest::Url;

use crate::fake_hub::{FakeHub, Reply};

fn token() -> daemon_core::wire::DaemonToken {
    daemon_core::wire::DaemonToken::from_random([5; 32])
}

fn batch() -> Batch {
    Batch::new(uuid::Uuid::from_u128(1), [], Announcements::default())
}

#[tokio::test]
async fn met_020_a_redirect_is_a_failed_request_and_is_not_followed() {
    let hub = FakeHub::start().await;
    hub.queue_push([Reply::Redirect("/daemon/v1/renew".to_owned())]);
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(answer, Answer::Failed(Cause::Status(302)));
    assert!(
        hub.seen("renew").is_empty(),
        "the redirect target was never requested"
    );
}

/// A folder that only this user can reach, as the folder of the socket of the link must be.
fn private_folder() -> tempfile::TempDir {
    let folder = tempfile::tempdir().expect("a folder");
    std::fs::set_permissions(folder.path(), Permissions::from_mode(0o700)).expect("mode");
    folder
}

#[tokio::test]
async fn k8_1_a_push_reaches_the_hub_through_the_socket_of_the_link() {
    let folder = private_folder();
    let socket = folder.path().join("link.sock");
    let hub = FakeHub::behind_socket(&socket);
    let origin = Origin::parse(&format!("unix:{}", socket.display())).expect("a link origin");
    let client = HubClient::reaching(origin.reach(), &folder.path().join("unused"), TIMEOUT)
        .expect("a client");
    assert_eq!(
        client.push(&token(), &batch()).await,
        Answer::Accepted(None)
    );
    assert_eq!(
        hub.seen("push")[0].bearer.as_deref(),
        Some(token().expose())
    );
}

/// R-172, SF-004: after its update, a daemon that was enrolled at a port of the link pushes
/// through the socket in its own folder, and the port gets no request and no token.
#[tokio::test]
async fn k8_1_a_daemon_with_the_old_link_origin_pushes_by_its_socket_and_sends_nothing_to_the_port()
{
    let on_the_port = FakeHub::start().await;
    let old = Origin::parse_kept(on_the_port.url.as_str().trim_end_matches('/'))
        .expect("a stored old link origin");
    let folder = private_folder();
    let own_socket = folder.path().join("link.sock");
    let behind_the_link = FakeHub::behind_socket(&own_socket);

    let client = HubClient::reaching(old.reach(), &own_socket, TIMEOUT).expect("a client");
    assert_eq!(
        client.push(&token(), &batch()).await,
        Answer::Accepted(None)
    );
    assert_eq!(behind_the_link.seen("push").len(), 1);
    assert!(
        on_the_port.seen("push").is_empty(),
        "the token went to a TCP port"
    );
}

#[tokio::test]
async fn k8_1_while_the_link_is_stopped_a_push_fails_and_goes_nowhere() {
    let folder = private_folder();
    let client = HubClient::over_link(&folder.path().join("link.sock"), TIMEOUT)
        .expect("only this user can make a socket here");
    assert!(matches!(
        client.push(&token(), &batch()).await,
        Answer::Failed(_)
    ));
}

#[test]
fn k8_1_no_client_is_made_for_a_socket_that_a_different_user_could_put_there() {
    let open = private_folder();
    std::fs::set_permissions(open.path(), Permissions::from_mode(0o755)).expect("mode");
    assert!(matches!(
        HubClient::over_link(&open.path().join("link.sock"), TIMEOUT),
        Err(HubError::LinkSocket(LinkSocketError::Exposed))
    ));

    let folder = private_folder();
    let file = folder.path().join("link.sock");
    std::fs::write(&file, b"").expect("a file");
    assert!(matches!(
        HubClient::over_link(&file, TIMEOUT),
        Err(HubError::LinkSocket(LinkSocketError::Exposed))
    ));

    let elsewhere = private_folder();
    let real = elsewhere.path().join("link.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&real).expect("a socket");
    let link = folder.path().join("linked.sock");
    std::os::unix::fs::symlink(&real, &link).expect("a link");
    assert!(matches!(
        HubClient::over_link(&link, TIMEOUT),
        Err(HubError::LinkSocket(LinkSocketError::Exposed))
    ));

    assert!(matches!(
        HubClient::over_link(&folder.path().join("none").join("link.sock"), TIMEOUT),
        Err(HubError::LinkSocket(LinkSocketError::Io(_)))
    ));
}

#[tokio::test]
async fn met_020_no_cookie_the_hub_sets_comes_back() {
    let hub = FakeHub::start().await;
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    client.push(&token(), &batch()).await;
    client.push(&token(), &batch()).await;
    assert!(hub.seen("push").iter().all(|seen| !seen.cookie));
}

#[tokio::test]
async fn k8_6_a_refusal_reads_through_the_client() {
    let hub = FakeHub::start().await;
    hub.queue_push([Reply::refuse(RefusalCode::WrongDevice)]);
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(answer, Answer::Refused(RefusalCode::WrongDevice));
}

/// The child half of the proxy test: it runs only when the parent started it with the hub's
/// address, and a proxy address in every proxy variable.
#[tokio::test]
async fn proxy_child() {
    let Ok(address) = std::env::var("ANIMA_TEST_HUB") else {
        return;
    };
    let client = HubClient::new(Url::parse(&address).expect("a URL"), TIMEOUT).expect("a client");
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(
        answer,
        Answer::Accepted(None),
        "the push must not go through a proxy"
    );
}

// Multi-threaded: the parent waits for the child while the fake hub answers it.
#[tokio::test(flavor = "multi_thread")]
async fn met_020_proxy_variables_are_ignored() {
    let hub = FakeHub::start().await;
    let dead = "http://127.0.0.1:9";
    let outcome = Command::new(std::env::current_exe().expect("this test binary"))
        .args(["--exact", "transport::proxy_child", "--nocapture"])
        .env("ANIMA_TEST_HUB", hub.url.as_str())
        .env("HTTPS_PROXY", dead)
        .env("https_proxy", dead)
        .env("HTTP_PROXY", dead)
        .env("http_proxy", dead)
        .env("ALL_PROXY", dead)
        .env("all_proxy", dead)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .output()
        .expect("the child runs");
    assert!(
        outcome.status.success(),
        "{}",
        String::from_utf8_lossy(&outcome.stdout)
    );
    assert_eq!(hub.seen("push").len(), 1, "the push reached the hub");
}

#[tokio::test]
async fn met_020_a_hub_that_accepts_and_never_answers_ends_in_a_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.expect("accept");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        drop(socket);
    });
    let url = Url::parse(&format!("http://{address}")).expect("url");
    let client = HubClient::new(url, std::time::Duration::from_millis(300)).expect("a client");
    let started = std::time::Instant::now();
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(answer, Answer::Failed(Cause::Timeout));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn k8_7_the_request_time_limit_is_10_seconds() {
    assert_eq!(
        anima_daemon::hub::TIMEOUT,
        std::time::Duration::from_secs(10)
    );
}

#[tokio::test]
async fn met_020_an_endless_answer_is_cut_at_the_limit_and_does_not_read() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut head = vec![0_u8; 4096];
        let _ = socket.read(&mut head).await;
        let head = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\r\n";
        socket.write_all(head.as_bytes()).await.ok();
        let chunk = vec![b' '; 1 << 20];
        loop {
            let frame = format!("{:x}\r\n", chunk.len());
            let wrote = socket.write_all(frame.as_bytes()).await.is_ok()
                && socket.write_all(&chunk).await.is_ok()
                && socket.write_all(b"\r\n").await.is_ok();
            if !wrote {
                break;
            }
        }
    });
    let url = Url::parse(&format!("http://{address}")).expect("url");
    let client = HubClient::new(url, TIMEOUT).expect("a client");
    let started = std::time::Instant::now();
    let answer = client.renew(&token()).await;
    assert!(matches!(answer, Answer::Failed(Cause::Unreadable)));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "the read stopped at the limit"
    );
}

#[tokio::test]
async fn k8_1_a_2xx_that_is_not_the_routes_success_status_is_a_failure() {
    let hub = FakeHub::start().await;
    // A push succeeds with 204, or with 200 when the hub names the keys (K-E5 Section 2.1).
    hub.queue_push([Reply::json(202, "{}")]);
    let client = HubClient::new(hub.url.clone(), TIMEOUT).expect("a client");
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(answer, Answer::Failed(Cause::Status(202)));
}

/// A hub that sends the headers of a 200 and then `body` after `pause`, with the length it
/// promises (K8 answers are small; the body is what the client waits for).
async fn slow_hub(promised: usize, body: Vec<u8>, pause: std::time::Duration) -> Url {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut head = vec![0_u8; 4096];
        let _ = socket.read(&mut head).await;
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {promised}\r\n\r\n"
        );
        socket.write_all(head.as_bytes()).await.ok();
        tokio::time::sleep(pause).await;
        socket.write_all(&body).await.ok();
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    });
    Url::parse(&format!("http://{address}")).expect("url")
}

#[tokio::test]
async fn met_020_a_body_that_trickles_in_after_the_headers_ends_in_a_timeout() {
    let url = slow_hub(100, b"{".to_vec(), std::time::Duration::from_secs(30)).await;
    let client = HubClient::new(url, std::time::Duration::from_millis(300)).expect("a client");
    let started = std::time::Instant::now();
    let answer = client.renew(&token()).await;
    assert!(matches!(answer, Answer::Failed(Cause::Timeout)));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

/// A renewal answer with `padding` spaces before the JSON, so a cut at the limit leaves only
/// spaces and cannot be read.
fn padded_renewal(padding: usize) -> Vec<u8> {
    let json = format!(
        r#"{{"token":"{}","expiresAt":"2030-01-01T00:00:00Z"}}"#,
        token().expose()
    );
    let mut body = vec![b' '; padding];
    body.extend_from_slice(json.as_bytes());
    body
}

#[tokio::test]
async fn met_020_an_answer_of_60_kib_is_read_and_one_of_70_kib_is_cut_and_does_not_read() {
    for (padding, accepted) in [(60 * 1024, true), (70 * 1024, false)] {
        let body = padded_renewal(padding);
        let url = slow_hub(body.len(), body, std::time::Duration::ZERO).await;
        let client = HubClient::new(url, TIMEOUT).expect("a client");
        let answer = client.renew(&token()).await;
        if accepted {
            assert!(matches!(answer, Answer::Accepted(_)), "{padding} bytes");
        } else {
            assert!(
                matches!(answer, Answer::Failed(Cause::Unreadable)),
                "{padding} bytes"
            );
        }
    }
}
