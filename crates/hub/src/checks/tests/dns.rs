//! DNS probe against a hand-written local UDP server.

use super::support::{LOOPBACK, dns_server, env, silent_dns_server, strict_env, target, wire_name};
use crate::checks::{
    config::{CheckConfig, DnsRecordType},
    probe::{ProbeEnv, ProbeResult, probe},
};

fn query(record_type: DnsRecordType, expected: Option<&str>) -> CheckConfig {
    CheckConfig::Dns {
        name: "nas.example.test".into(),
        record_type,
        resolver: Some(LOOPBACK),
        expected: expected.map(str::to_owned),
    }
}

async fn run(config: &CheckConfig, port: u16) -> ProbeResult {
    let mut env: ProbeEnv = env();
    env.overrides.dns_port = Some(port);
    probe(&target("unused"), config, None, &env).await
}

#[tokio::test]
async fn chk_001_dns_any_answer_is_up_without_an_expected_value() {
    let port = dns_server(1, vec![192, 168, 1, 10]).await;
    let result = run(&query(DnsRecordType::A, None), port).await;
    assert!(result.ok, "{:?}", result.message);
    assert!(result.response_time.is_some());
}

#[tokio::test]
async fn chk_001_dns_answer_must_equal_the_expected_value() {
    let port = dns_server(1, vec![192, 168, 1, 10]).await;
    assert!(
        run(&query(DnsRecordType::A, Some("192.168.1.10")), port)
            .await
            .ok
    );
    let wrong = run(&query(DnsRecordType::A, Some("192.168.1.11")), port).await;
    assert!(!wrong.ok);
    assert_eq!(
        wrong.message.as_deref(),
        Some("the DNS answer is not the expected one")
    );
}

#[tokio::test]
async fn chk_001_dns_comparison_ignores_case_and_a_trailing_dot() {
    let port = dns_server(5, wire_name("Target.Example.Test.")).await;
    for expected in [
        "target.example.test",
        "TARGET.EXAMPLE.TEST.",
        "Target.Example.Test",
    ] {
        let result = run(&query(DnsRecordType::Cname, Some(expected)), port).await;
        assert!(result.ok, "{expected}: {:?}", result.message);
    }
    assert!(
        !run(
            &query(DnsRecordType::Cname, Some("other.example.test")),
            port
        )
        .await
        .ok
    );
}

#[tokio::test]
async fn chk_001_dns_txt_and_mx_answers_are_compared_as_text() {
    let txt = dns_server(16, [&[11u8][..], b"hello world"].concat()).await;
    assert!(
        run(&query(DnsRecordType::Txt, Some("hello world")), txt)
            .await
            .ok
    );
    let mx = dns_server(
        15,
        [&[0u8, 10][..], &wire_name("mail.example.test")].concat(),
    )
    .await;
    assert!(
        run(&query(DnsRecordType::Mx, Some("mail.example.test")), mx)
            .await
            .ok
    );
    assert!(
        run(&query(DnsRecordType::Mx, Some("10 mail.example.test.")), mx)
            .await
            .ok
    );
    assert!(
        !run(&query(DnsRecordType::Mx, Some("20 mail.example.test")), mx)
            .await
            .ok
    );
}

#[tokio::test]
async fn chk_001_dns_only_answers_of_the_asked_type_count() {
    // The server answers a CNAME to an A question; that is not an A answer.
    let port = dns_server(5, wire_name("target.example.test")).await;
    let result = run(&query(DnsRecordType::A, None), port).await;
    assert!(!result.ok);
}

#[tokio::test]
async fn chk_001_dns_server_that_never_answers_fails_within_the_limit() {
    let port = silent_dns_server().await;
    let started = std::time::Instant::now();
    let config = CheckConfig::Dns {
        name: "nas.example.test".into(),
        record_type: DnsRecordType::A,
        resolver: Some(LOOPBACK),
        expected: None,
    };
    let result = run(&config, port).await;
    assert!(!result.ok);
    assert!(started.elapsed() < std::time::Duration::from_secs(8));
}

#[tokio::test]
async fn dns_resolver_address_is_checked_by_the_policy() {
    let port = dns_server(1, vec![1, 2, 3, 4]).await;
    let mut env = strict_env();
    env.overrides.dns_port = Some(port);
    let result = probe(
        &target("unused"),
        &query(DnsRecordType::A, None),
        None,
        &env,
    )
    .await;
    assert_eq!(
        result.message.as_deref(),
        Some("the address is not permitted")
    );
    for refused in ["169.254.169.254", "::ffff:127.0.0.1", "0.0.0.0"] {
        let config = CheckConfig::Dns {
            name: "a.test".into(),
            record_type: DnsRecordType::A,
            resolver: Some(refused.parse().unwrap()),
            expected: None,
        };
        let result = probe(&target("unused"), &config, None, &env).await;
        assert_eq!(
            result.message.as_deref(),
            Some("the address is not permitted"),
            "{refused}"
        );
    }
}
