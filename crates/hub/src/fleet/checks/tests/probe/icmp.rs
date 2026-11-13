//! ICMP probe. Unprivileged ICMP sockets may be unavailable (sandbox, Linux
//! `ping_group_range`); then the probe must fail with a clear message instead of panicking.

use std::time::Duration;

use super::support::{LOOPBACK, env, strict_env, target};
use crate::fleet::checks::{
    config::{CheckConfig, IpVersion},
    probe::{Failure, ProbeResult, probe, within},
};

fn icmp(version: IpVersion) -> CheckConfig {
    CheckConfig::Icmp {
        timeout_ms: 1500,
        ip_version: version,
    }
}

#[tokio::test]
async fn chk_012_icmp_to_loopback_succeeds_or_reports_an_unavailable_socket() {
    let result = probe(&target("127.0.0.1"), &icmp(IpVersion::V4), None, &env()).await;
    if result.ok {
        assert_eq!(result.resolved_ip, Some(LOOPBACK));
        assert!(result.response_time.is_some());
    } else {
        let message = result.message.unwrap();
        assert!(
            message.starts_with("the ICMP socket is unavailable") || message == "no ICMP reply",
            "{message}"
        );
    }
}

#[tokio::test]
async fn icmp_version_chooses_the_address_family() {
    let mut env = env();
    env.overrides.hosts =
        std::collections::HashMap::from([("six.test".to_owned(), vec!["::1".parse().unwrap()])])
            .into();
    let v4_only = probe(&target("six.test"), &icmp(IpVersion::V4), None, &env).await;
    assert_eq!(
        v4_only.message.as_deref(),
        Some("the host has no address of the requested version")
    );
    let v6_literal = probe(&target("127.0.0.1"), &icmp(IpVersion::V6), None, &env).await;
    assert_eq!(
        v6_literal.message.as_deref(),
        Some("the host has no address of the requested version")
    );
}

#[tokio::test]
async fn icmp_default_policy_refuses_loopback() {
    let result = probe(
        &target("127.0.0.1"),
        &icmp(IpVersion::V4),
        None,
        &strict_env(),
    )
    .await;
    assert_eq!(
        result.message.as_deref(),
        Some("the address is not permitted")
    );
}

/// A pinger that gives up exactly at the check's limit, racing the probe's own timer.
async fn pinger_giving_up_at(limit: Duration) -> ProbeResult {
    tokio::time::sleep(limit).await;
    ProbeResult::failed(&Failure::NoReply)
}

#[tokio::test(start_paused = true)]
async fn chk_001_an_echo_with_no_reply_at_the_limit_always_says_no_icmp_reply() {
    let config = icmp(IpVersion::V4);
    for round in 0..200 {
        let racing = within(&config, pinger_giving_up_at(config.timeout())).await;
        let silent = within(&config, std::future::pending()).await;
        assert_eq!(
            racing.message.as_deref(),
            Some("no ICMP reply"),
            "round {round}"
        );
        assert_eq!(
            silent.message.as_deref(),
            Some("no ICMP reply"),
            "round {round}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn chk_001_other_checks_still_time_out_at_their_limit() {
    let tcp = CheckConfig::Tcp {
        port: 22,
        timeout_ms: 1500,
    };
    let result = within(&tcp, std::future::pending()).await;
    assert_eq!(result.message.as_deref(), Some("the check timed out"));
}
