//! The transport rules of K8 Section 1 (MET-020): no redirect, no proxy, no cookie.

use std::process::Command;

use agent_core::wire::{Batch, RefusalCode};
use anima_agent::hub::HubClient;
use anima_agent::hub::answer::{Answer, Cause};
use reqwest::Url;

use crate::fake_hub::{FakeHub, Reply};

fn token() -> agent_core::wire::AgentToken {
    agent_core::wire::AgentToken::from_random([5; 32])
}

fn batch() -> Batch {
    Batch::new(uuid::Uuid::from_u128(1), [], None)
}

#[tokio::test]
async fn met_020_a_redirect_is_a_failed_request_and_is_not_followed() {
    let hub = FakeHub::start().await;
    hub.queue_push([Reply::Redirect("/agent/v1/renew".to_owned())]);
    let client = HubClient::new(hub.url.clone()).expect("a client");
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(answer, Answer::Failed(Cause::Status(302)));
    assert!(
        hub.seen("renew").is_empty(),
        "the redirect target was never requested"
    );
}

#[tokio::test]
async fn met_020_no_cookie_the_hub_sets_comes_back() {
    let hub = FakeHub::start().await;
    let client = HubClient::new(hub.url.clone()).expect("a client");
    client.push(&token(), &batch()).await;
    client.push(&token(), &batch()).await;
    assert!(hub.seen("push").iter().all(|seen| !seen.cookie));
}

#[tokio::test]
async fn k8_6_a_refusal_reads_through_the_client() {
    let hub = FakeHub::start().await;
    hub.queue_push([Reply::refuse(RefusalCode::WrongDevice)]);
    let client = HubClient::new(hub.url.clone()).expect("a client");
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
    let client = HubClient::new(Url::parse(&address).expect("a URL")).expect("a client");
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(
        answer,
        Answer::Accepted(()),
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
    let client =
        HubClient::with_timeout(url, std::time::Duration::from_millis(300)).expect("a client");
    let started = std::time::Instant::now();
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(answer, Answer::Failed(Cause::Timeout));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn k8_7_the_request_time_limit_is_10_seconds() {
    assert_eq!(
        anima_agent::hub::TIMEOUT,
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
    let client = HubClient::new(url).expect("a client");
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
    hub.queue_push([Reply::json(200, "{}")]);
    let client = HubClient::new(hub.url.clone()).expect("a client");
    let answer = client.push(&token(), &batch()).await;
    assert_eq!(answer, Answer::Failed(Cause::Status(200)));
}
