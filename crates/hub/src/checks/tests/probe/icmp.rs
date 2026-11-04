//! ICMP probe. Unprivileged ICMP sockets may be unavailable (sandbox, Linux
//! `ping_group_range`); then the probe must fail with a clear message instead of panicking.

use super::support::{LOOPBACK, env, strict_env, target};
use crate::checks::{
    config::{CheckConfig, IpVersion},
    probe::probe,
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
