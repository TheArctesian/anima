//! TLS probe against local listeners with generated self-signed certificates (CHK-007).

use chrono::{TimeZone, Utc};

use super::support::{LOOPBACK, Reply, acceptor, env, serve, target};
use crate::fleet::checks::{config::CheckConfig, probe::probe};

fn tls(port: u16, server_name: Option<&str>) -> CheckConfig {
    CheckConfig::Tls {
        port,
        server_name: server_name.map(str::to_owned),
        timeout_ms: 3000,
    }
}

#[tokio::test]
async fn chk_007_valid_self_signed_certificate_is_up_and_reports_its_expiry() {
    let server = serve(Some(acceptor(2020, 2040)), |_| Reply::ok("")).await;
    let result = probe(
        &target("127.0.0.1"),
        &tls(server.addr.port(), None),
        None,
        &env(),
    )
    .await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(
        result.cert_not_after,
        Some(Utc.with_ymd_and_hms(2040, 1, 1, 0, 0, 0).unwrap())
    );
    assert_eq!(result.resolved_ip, Some(LOOPBACK));
    assert!(result.response_time.is_some());
}

#[tokio::test]
async fn chk_007_expired_certificate_fails_but_still_reports_the_date() {
    let server = serve(Some(acceptor(2018, 2021)), |_| Reply::ok("")).await;
    let result = probe(
        &target("127.0.0.1"),
        &tls(server.addr.port(), None),
        None,
        &env(),
    )
    .await;
    assert!(!result.ok);
    assert_eq!(
        result.message.as_deref(),
        Some("the certificate has expired")
    );
    assert_eq!(
        result.cert_not_after,
        Some(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap())
    );
}

#[tokio::test]
async fn chk_007_certificate_not_valid_yet_fails() {
    let server = serve(Some(acceptor(2035, 2040)), |_| Reply::ok("")).await;
    let result = probe(
        &target("127.0.0.1"),
        &tls(server.addr.port(), None),
        None,
        &env(),
    )
    .await;
    assert!(!result.ok);
    assert_eq!(
        result.message.as_deref(),
        Some("the certificate is not valid yet")
    );
}

#[tokio::test]
async fn chk_007_validity_is_judged_by_the_now_parameter() {
    let server = serve(Some(acceptor(2020, 2030)), |_| Reply::ok("")).await;
    let config = tls(server.addr.port(), None);
    let mut later = env();
    later.now = Utc.with_ymd_and_hms(2031, 1, 1, 0, 0, 0).unwrap();
    assert!(probe(&target("127.0.0.1"), &config, None, &env()).await.ok);
    assert!(!probe(&target("127.0.0.1"), &config, None, &later).await.ok);
}

#[tokio::test]
async fn tls_to_a_plain_tcp_service_fails_the_handshake() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut tcp, _)) = listener.accept().await {
            let _ = tcp.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
        }
    });
    let result = probe(&target("127.0.0.1"), &tls(port, None), None, &env()).await;
    assert!(!result.ok);
    assert_eq!(result.message.as_deref(), Some("the TLS handshake failed"));
    assert_eq!(result.resolved_ip, Some(LOOPBACK));
}

#[tokio::test]
async fn tls_closed_port_fails_to_connect() {
    let port = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let result = probe(&target("127.0.0.1"), &tls(port, None), None, &env()).await;
    assert!(!result.ok);
    assert!(result.message.unwrap().starts_with("the connection failed"));
}

#[tokio::test]
async fn tls_server_name_overrides_the_host_for_sni() {
    use std::sync::{Arc, Mutex};
    let acceptor = acceptor(2020, 2040);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen: Arc<Mutex<Vec<Option<String>>>> = Arc::default();
    let record = seen.clone();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            if let Ok(stream) = acceptor.accept(tcp).await {
                record
                    .lock()
                    .unwrap()
                    .push(stream.get_ref().1.server_name().map(str::to_owned));
            }
        }
    });
    let named = probe(
        &target("127.0.0.1"),
        &tls(port, Some("nas.example.test")),
        None,
        &env(),
    )
    .await;
    assert!(named.ok, "{:?}", named.message);
    let mut env = env();
    env.overrides.hosts =
        std::collections::HashMap::from([("host.test".to_owned(), vec![LOOPBACK])]).into();
    assert!(
        probe(&target("host.test"), &tls(port, None), None, &env)
            .await
            .ok
    );
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let seen = seen.lock().unwrap().clone();
    assert_eq!(
        seen,
        vec![
            Some("nas.example.test".to_owned()),
            Some("host.test".to_owned())
        ]
    );
}

#[tokio::test]
async fn tls_listener_that_accepts_and_stays_silent_fails_within_the_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((tcp, _)) = listener.accept().await {
            held.push(tcp);
        }
    });
    let config = CheckConfig::Tls {
        port,
        server_name: None,
        timeout_ms: 300,
    };
    let started = std::time::Instant::now();
    // The guard turns "no outer timeout" into a failure instead of a hang.
    let guarded = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        probe(&target("127.0.0.1"), &config, None, &env()),
    )
    .await
    .expect("the probe must stop at its own timeout");
    assert!(!guarded.ok);
    assert_eq!(guarded.message.as_deref(), Some("the check timed out"));
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    assert!(started.elapsed() >= std::time::Duration::from_millis(250));
}
