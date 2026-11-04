//! The probe's edge cases that need no SSH server (R-032, D-5): refused targets, a closed
//! port, a host that never answers and one that does not speak SSH. The key exchange and the
//! sign-in run against an in-process server in `tests/it/sessions/host_keys`.

use std::{net::IpAddr, time::Duration};

use tokio::{io::AsyncWriteExt, net::TcpListener};

use crate::sessions::{
    probe::{ProbeError, Target, read_host_key},
    target::TargetRule,
};

const SHORT: Duration = Duration::from_millis(300);

fn strict(host: &str, port: u16) -> Target<'_> {
    Target {
        host,
        port,
        rule: TargetRule::STRICT,
    }
}

fn loopback(port: u16) -> Target<'static> {
    Target {
        host: "127.0.0.1",
        port,
        rule: TargetRule::LOOPBACK_FOR_TESTS,
    }
}

#[tokio::test]
async fn r_032_the_probe_refuses_a_host_that_resolves_to_the_gateway_itself() {
    // A listener proves the refusal comes before any connection: nothing is accepted.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    for host in [
        "localhost",
        "127.0.0.1",
        "::1",
        "::ffff:127.0.0.1",
        "0.0.0.0",
    ] {
        assert_eq!(
            read_host_key(strict(host, port), SHORT).await,
            Err(ProbeError::RefusedTarget),
            "{host}"
        );
    }
    for host in ["169.254.169.254", "fe80::1", "224.0.0.1", "64:ff9b::7f00:1"] {
        assert_eq!(
            read_host_key(strict(host, 22), SHORT).await,
            Err(ProbeError::RefusedTarget),
            "{host}"
        );
    }
    let accepted = tokio::time::timeout(Duration::from_millis(50), listener.accept()).await;
    assert!(accepted.is_err(), "the probe never connected");
}

#[test]
fn r_032_the_test_rule_lets_only_plain_loopback_through() {
    let rule = TargetRule::LOOPBACK_FOR_TESTS;
    for allowed in ["127.0.0.1", "127.0.0.2", "::1", "10.10.0.2"] {
        assert!(
            !rule.refuses(allowed.parse::<IpAddr>().unwrap()),
            "{allowed}"
        );
    }
    for refused in [
        "::ffff:127.0.0.1",
        "169.254.1.1",
        "fe80::1",
        "0.0.0.0",
        "ff02::1",
    ] {
        assert!(
            rule.refuses(refused.parse::<IpAddr>().unwrap()),
            "{refused}"
        );
    }
    for refused in ["127.0.0.1", "::1"] {
        let ip = refused.parse::<IpAddr>().unwrap();
        assert!(TargetRule::STRICT.refuses(ip), "{refused}");
    }
}

#[tokio::test]
async fn dev_013_a_closed_port_is_unreachable() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    };
    assert_eq!(
        read_host_key(loopback(port), SHORT).await,
        Err(ProbeError::Unreachable)
    );
}

#[tokio::test]
async fn dev_013_a_host_that_never_answers_times_out_within_the_limit() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let silent = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        drop(stream);
    });
    let started = std::time::Instant::now();
    assert_eq!(
        read_host_key(loopback(port), SHORT).await,
        Err(ProbeError::Timeout)
    );
    assert!(started.elapsed() < SHORT * 3, "{:?}", started.elapsed());
    silent.abort();
}

#[tokio::test]
async fn dev_013_a_host_that_does_not_speak_ssh_is_a_protocol_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let http = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\nServer: banner-text\r\n\r\n")
            .await
            .unwrap();
    });
    assert_eq!(
        read_host_key(loopback(port), Duration::from_secs(5)).await,
        Err(ProbeError::Protocol)
    );
    http.await.unwrap();
}
