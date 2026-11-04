//! HTTP probe against local servers (CHK-001, CHK-007).

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use chrono::{TimeZone, Utc};

use super::support::{Body, LOOPBACK, Reply, acceptor, env, serve, target};
use crate::checks::{
    config::{CheckConfig, HttpConfig, HttpHeader, StatusRange},
    probe::{BODY_CAP, ProbeResult, probe},
};

async fn run(config: HttpConfig) -> ProbeResult {
    probe(&target("unused"), &CheckConfig::Http(config), None, &env()).await
}

fn message(result: &ProbeResult) -> &str {
    result.message.as_deref().unwrap_or("")
}

#[tokio::test]
async fn chk_001_http_success_reports_time_address_and_sends_the_agent() {
    let server = serve(None, |_| Reply::ok("hello")).await;
    let result = run(HttpConfig::new(server.url("/x"))).await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(result.resolved_ip, Some(LOOPBACK));
    assert!(result.response_time.is_some());
    assert_eq!(result.cert_not_after, None);
    let request = &server.seen()[0];
    assert_eq!(request.path, "/x");
    assert_eq!(request.header("user-agent"), Some("anima-hub"));
    assert_eq!(request.header("cookie"), None);
}

#[tokio::test]
async fn chk_001_http_status_must_be_inside_an_accepted_range() {
    let server = serve(None, |r| match r.path.as_str() {
        "/ok" => Reply::status(204, ""),
        "/gone" => Reply::status(503, "x"),
        _ => Reply::status(404, ""),
    })
    .await;
    assert!(run(HttpConfig::new(server.url("/ok"))).await.ok);
    let bad = run(HttpConfig::new(server.url("/gone"))).await;
    assert!(!bad.ok);
    assert_eq!(message(&bad), "unexpected status 503");
    assert_eq!(bad.resolved_ip, Some(LOOPBACK));
    let mut accept_errors = HttpConfig::new(server.url("/gone"));
    accept_errors.accepted_status = vec![StatusRange { from: 500, to: 503 }];
    assert!(run(accept_errors).await.ok);
    let mut edge = HttpConfig::new(server.url("/gone"));
    edge.accepted_status = vec![
        StatusRange { from: 200, to: 299 },
        StatusRange { from: 504, to: 599 },
    ];
    assert!(!run(edge).await.ok);
}

#[tokio::test]
async fn chk_001_http_keyword_must_be_in_the_body() {
    let server = serve(None, |_| Reply::ok("service ready: all good")).await;
    let mut config = HttpConfig::new(server.url("/"));
    config.keyword = Some("all good".into());
    assert!(run(config.clone()).await.ok);
    config.keyword = Some("all bad".into());
    let result = run(config).await;
    assert!(!result.ok);
    assert!(message(&result).starts_with("the keyword was not found"));
    assert!(!message(&result).contains("service ready"));
}

#[tokio::test]
async fn chk_001_http_json_pointer_value_must_equal_the_expected_text() {
    let body = r#"{"status":"ok","n":3,"f":3.0,"flag":true,"deep":{"list":[{"v":"x"}]}}"#;
    let server = serve(None, move |r| {
        if r.path == "/text" {
            Reply::ok("not json")
        } else {
            Reply::ok(body)
        }
    })
    .await;
    let json = |query: &str, expected: Option<&str>| {
        let mut config = HttpConfig::new(server.url("/"));
        config.json_query = Some(query.into());
        config.json_expected = expected.map(str::to_owned);
        config
    };
    assert!(run(json("/status", Some("ok"))).await.ok);
    assert!(run(json("/n", Some("3"))).await.ok);
    assert!(run(json("/flag", Some("true"))).await.ok);
    assert!(run(json("/deep/list/0/v", Some("x"))).await.ok);
    assert!(run(json("/status", None)).await.ok);
    // Equal length, a prefix, and the rendering of a float (serde: 3.0 stays "3.0").
    assert!(!run(json("/status", Some("ox"))).await.ok);
    assert!(!run(json("/status", Some("o"))).await.ok);
    assert!(!run(json("/status", Some("okk"))).await.ok);
    assert!(!run(json("/f", Some("3"))).await.ok);
    assert!(run(json("/f", Some("3.0"))).await.ok);
    assert_eq!(
        message(&run(json("/status", Some("bad"))).await),
        "the JSON value is not the expected one"
    );
    assert_eq!(
        message(&run(json("/nothing", None)).await),
        "the JSON query found no value"
    );
    let mut not_json = json("/status", Some("ok"));
    not_json.url = server.url("/text");
    assert_eq!(message(&run(not_json).await), "the body is not JSON");
}

#[tokio::test]
async fn chk_001_http_method_and_body_reach_the_server() {
    let server = serve(None, |r| {
        if r.header("x-check") == Some("1") {
            Reply::ok("")
        } else {
            Reply::status(400, "")
        }
    })
    .await;
    let mut config = HttpConfig::new(server.url("/"));
    config.method = crate::checks::config::HttpMethod::Post;
    config.body = Some("{}".into());
    config.headers = vec![HttpHeader {
        name: "X-Check".into(),
        value: Some("1".into()),
    }];
    assert!(run(config).await.ok);
}

#[tokio::test]
async fn chk_001_http_redirects_are_followed_up_to_the_limit() {
    let server = serve(None, |r| {
        let n: u32 = r.path.trim_start_matches('/').parse().unwrap_or(0);
        if n < 3 {
            Reply::redirect(&format!("/{}", n + 1))
        } else {
            Reply::ok("end")
        }
    })
    .await;
    let mut config = HttpConfig::new(server.url("/0"));
    config.max_redirects = 3;
    assert!(run(config.clone()).await.ok);
    config.max_redirects = 2;
    let result = run(config.clone()).await;
    assert!(!result.ok);
    assert_eq!(message(&result), "too many redirects");
    config.max_redirects = 0;
    assert_eq!(message(&run(config).await), "unexpected status 302");
}

#[tokio::test]
async fn chk_001_redirect_to_a_refused_address_is_refused() {
    let target_server = serve(None, |_| Reply::ok("secret")).await;
    let port = target_server.addr.port();
    let mut env = env();
    env.overrides.hosts = std::collections::HashMap::from([(
        "meta.test".to_owned(),
        vec!["169.254.169.254".parse().unwrap()],
    )])
    .into();
    let locations = [
        "http://169.254.169.254/latest/meta-data/".to_owned(),
        format!("http://[::ffff:169.254.169.254]:{port}/"),
        "http://meta.test/".to_owned(),
        "http://0.0.0.0/".to_owned(),
        "ftp://127.0.0.1/".to_owned(),
    ];
    for location in locations {
        let to = location.clone();
        let server = serve(None, move |_| Reply::redirect(&to)).await;
        let result = probe(
            &target("x"),
            &CheckConfig::Http(HttpConfig::new(server.url("/"))),
            None,
            &env,
        )
        .await;
        assert!(!result.ok, "{location}");
        assert_eq!(
            message(&result),
            "the address is not permitted",
            "{location}"
        );
    }
    assert_eq!(target_server.connections.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn chk_001_http_body_is_not_read_beyond_one_mebibyte() {
    let written = Arc::new(AtomicUsize::new(0));
    let total = 256 * 1024 * 1024;
    let counter = written.clone();
    let server = serve(None, move |_| {
        let mut prefix = vec![b'a'; BODY_CAP];
        prefix.extend_from_slice(b"NEEDLE");
        Reply {
            head: format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
            ),
            body: Body::Stream {
                prefix,
                fill: b'b',
                len: total,
                written: counter.clone(),
            },
        }
    })
    .await;
    let mut config = HttpConfig::new(server.url("/"));
    config.keyword = Some("NEEDLE".into());
    let result = run(config.clone()).await;
    assert!(!result.ok);
    assert!(message(&result).starts_with("the keyword was not found"));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        written.load(Ordering::SeqCst) < total / 4,
        "wrote {}",
        written.load(Ordering::SeqCst)
    );
    // A keyword inside the first mebibyte is found in the same kind of body.
    config.keyword = Some("aaaa".into());
    assert!(run(config).await.ok);
}

#[tokio::test]
async fn chk_001_http_without_body_checks_does_not_read_the_body() {
    let written = Arc::new(AtomicUsize::new(0));
    let counter = written.clone();
    let total = 64 * 1024 * 1024;
    let server = serve(None, move |_| Reply {
        head: format!("HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"),
        body: Body::Stream {
            prefix: Vec::new(),
            fill: b'x',
            len: total,
            written: counter.clone(),
        },
    })
    .await;
    assert!(run(HttpConfig::new(server.url("/"))).await.ok);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(written.load(Ordering::SeqCst) < total / 2);
}

#[tokio::test]
async fn chk_001_http_times_out_at_timeout_ms() {
    let server = serve(None, |_| Reply {
        head: String::new(),
        body: Body::Hang,
    })
    .await;
    let mut config = HttpConfig::new(server.url("/"));
    config.timeout_ms = 300;
    let started = Instant::now();
    let result = run(config).await;
    assert!(!result.ok);
    assert_eq!(message(&result), "the check timed out");
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(started.elapsed() >= Duration::from_millis(250));
}

#[tokio::test]
async fn chk_007_https_reports_the_certificate_expiry_and_verify_tls_controls_trust() {
    let server = serve(Some(acceptor(2020, 2040)), |_| Reply::ok("secure")).await;
    let url = format!("https://localhost:{}/", server.addr.port());
    let mut env = env();
    env.overrides.hosts =
        std::collections::HashMap::from([("localhost".to_owned(), vec![LOOPBACK])]).into();
    let mut config = HttpConfig::new(url);
    config.verify_tls = false;
    let result = probe(&target("x"), &CheckConfig::Http(config.clone()), None, &env).await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(
        result.cert_not_after,
        Some(Utc.with_ymd_and_hms(2040, 1, 1, 0, 0, 0).unwrap())
    );
    config.verify_tls = true;
    let strict = probe(&target("x"), &CheckConfig::Http(config), None, &env).await;
    assert!(
        !strict.ok,
        "a self-signed certificate must fail when verification is on"
    );
}

const PROXY_CHILD: &str = "ANIMA_TEST_PROXY_CHILD";

/// Runs only inside the child process of `proxy_variables_in_the_environment_are_ignored`.
#[tokio::test]
async fn proxy_environment_child() {
    if std::env::var_os(PROXY_CHILD).is_none() {
        return;
    }
    let server = serve(None, |_| Reply::ok("direct")).await;
    let result = run(HttpConfig::new(server.url("/"))).await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(server.seen().len(), 1);
}

#[test]
fn proxy_variables_in_the_environment_are_ignored() {
    let proxy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    proxy.set_nonblocking(true).unwrap();
    let url = format!("http://{}", proxy.local_addr().unwrap());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "checks::tests::http::proxy_environment_child",
            "--exact",
            "--test-threads=1",
        ])
        .env(PROXY_CHILD, "1")
        .env("HTTP_PROXY", &url)
        .env("http_proxy", &url)
        .env("ALL_PROXY", &url)
        .env("all_proxy", &url)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .status()
        .unwrap();
    assert!(
        status.success(),
        "the child probe went through the proxy or failed"
    );
    assert!(proxy.accept().is_err(), "the proxy received a connection");
}
