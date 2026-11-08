//! The address policy and the probes that must obey it (CHK-001).

use std::net::IpAddr;

use super::support::{Reply, env, serve, strict_env, target};
use crate::checks::{
    config::{CheckConfig, HttpConfig},
    probe::{Failure, ProbeResult, probe},
};
use crate::targets::AddressPolicy;

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

const REFUSED: &[&str] = &[
    "127.0.0.1",
    "127.255.255.254",
    "::1",
    "0.0.0.0",
    "0.1.2.3",
    "::",
    "169.254.169.254",
    "169.254.0.1",
    "fe80::1",
    "febf::1",
    "224.0.0.1",
    "239.255.255.255",
    "ff02::1",
    "255.255.255.255",
    "240.0.0.1",
    "::ffff:127.0.0.1",
    "::ffff:169.254.169.254",
    "::ffff:0.0.0.0",
    "::ffff:224.0.0.1",
    "::127.0.0.1",
    "::169.254.169.254",
    "64:ff9b::7f00:1",
    "64:ff9b::a9fe:a9fe",
    "2002:7f00:1::1",
    "fd00:ec2::254",
    "::ffff:0:7f00:1",
    "::ffff:0:a9fe:a9fe",
    "64:ff9b:1:7f00:0:100::",
    "64:ff9b:1:a9fe:a9:fe00::",
    "2001:0:4136:e378:8000:63bf:3fff:fdd2",
    "2001::1",
    "fec0::1",
    "feff::1",
    // Tailscale's quad-100 range and the 100.100.100.200 metadata address (CHK-012).
    "100.100.100.0",
    "100.100.100.200",
    "100.100.100.255",
    "::ffff:100.100.100.200",
    "::100.100.100.200",
    "64:ff9b::6464:64c8",
    "2002:6464:64c8::1",
    "::ffff:0:6464:64c8",
    // The whole local-use NAT64 /48, whatever it embeds.
    "64:ff9b:1:abcd::a9fe:a9fe",
    "64:ff9b:1:808:8:800::",
    "64:ff9b:1:ffff:ffff:ffff:ffff:ffff",
    "64:ff9b:1::",
    // Azure's wire server and the IETF protocol assignments 192.0.0.0/24.
    "168.63.129.16",
    "::ffff:168.63.129.16",
    "64:ff9b::a83f:8110",
    "2002:a83f:8110::1",
    "192.0.0.1",
    "192.0.0.192",
    "::ffff:192.0.0.8",
    "64:ff9b::c000:8",
];

const ALLOWED: &[&str] = &[
    "10.0.0.1",
    "172.16.0.1",
    "172.31.255.255",
    "192.168.1.1",
    "100.64.0.1",
    "100.127.255.255",
    "fc00::1",
    "fd12:3456::1",
    "8.8.8.8",
    "2606:4700::1111",
    "::ffff:10.0.0.1",
    "::ffff:192.168.1.1",
    "169.253.0.1",
    "169.255.0.1",
    "126.255.255.255",
    "223.255.255.255",
    "::ffff:0:a00:1",
    "100.100.99.255",
    "100.100.101.0",
    "100.99.100.200",
    "168.63.129.15",
    "168.63.129.17",
    "192.0.1.1",
    "64:ff9b:2::1",
    "2001:db8::1",
];

#[test]
fn default_policy_refuses_loopback_link_local_multicast_and_mapped_forms() {
    let policy = AddressPolicy::default();
    for text in REFUSED {
        assert!(!policy.permits(ip(text)), "{text} must be refused");
    }
}

#[test]
fn default_policy_allows_private_and_public_addresses() {
    let policy = AddressPolicy::default();
    for text in ALLOWED {
        assert!(policy.permits(ip(text)), "{text} must be allowed");
    }
}

#[test]
fn test_policy_allows_plain_loopback_only() {
    let policy = AddressPolicy::default().allowing_loopback();
    for text in ["127.0.0.1", "::1"] {
        assert!(policy.permits(ip(text)), "{text}");
    }
    // An IPv6 form that carries a loopback address stays refused (Q2-09).
    for text in [
        "::ffff:127.0.0.1",
        "169.254.169.254",
        "0.0.0.0",
        "224.0.0.1",
        "fe80::1",
        "::ffff:169.254.1.1",
    ] {
        assert!(!policy.permits(ip(text)), "{text}");
    }
}

#[test]
fn extra_deny_list_refuses_more_including_mapped_forms() {
    let deny = vec![
        "172.18.0.0/16".parse().unwrap(),
        "fd00:dead::/32".parse().unwrap(),
    ];
    let policy = AddressPolicy::new(deny);
    for text in ["172.18.0.5", "::ffff:172.18.0.5", "fd00:dead::1"] {
        assert!(!policy.permits(ip(text)), "{text}");
    }
    for text in ["172.17.0.5", "172.19.0.1", "fd00:beef::1", "10.0.0.1"] {
        assert!(policy.permits(ip(text)), "{text}");
    }
    // Even with loopback allowed, the deny list still applies.
    let both = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]).allowing_loopback();
    assert!(!both.permits(ip("127.0.0.1")));
}

fn assert_refused(result: &ProbeResult) {
    assert!(!result.ok);
    assert_eq!(
        result.message.as_deref(),
        Some("the address is not permitted")
    );
    assert_eq!(result.resolved_ip, None);
}

fn http(url: &str) -> CheckConfig {
    CheckConfig::Http(HttpConfig::new(url))
}

#[tokio::test]
async fn default_policy_refuses_http_to_literal_addresses_without_connecting() {
    let server = serve(None, |_| Reply::ok("hi")).await;
    let port = server.addr.port();
    let urls = [
        format!("http://127.0.0.1:{port}/"),
        format!("http://[::1]:{port}/"),
        format!("http://[::ffff:127.0.0.1]:{port}/"),
        format!("http://2130706433:{port}/"),
        format!("http://0x7f.1:{port}/"),
        "http://169.254.169.254/latest/meta-data/".to_owned(),
    ];
    for url in urls {
        let result = probe(&target("unused"), &http(&url), None, &strict_env()).await;
        assert_refused(&result);
    }
    assert_eq!(
        server.connections.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn default_policy_refuses_a_host_name_that_resolves_to_a_refused_address() {
    let server = serve(None, |_| Reply::ok("hi")).await;
    let mut env = strict_env();
    let hosts = [
        ("loop.test", "127.0.0.1"),
        ("meta.test", "169.254.169.254"),
        ("mapped.test", "::ffff:127.0.0.1"),
    ];
    env.overrides.hosts = hosts
        .iter()
        .map(|(n, a)| ((*n).to_owned(), vec![ip(a)]))
        .collect::<std::collections::HashMap<_, _>>()
        .into();
    for (name, _) in hosts {
        let url = format!("http://{name}:{}/", server.addr.port());
        assert_refused(&probe(&target("unused"), &http(&url), None, &env).await);
    }
    assert_eq!(
        server.connections.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn a_host_with_one_refused_and_one_allowed_address_connects_only_to_the_allowed_one() {
    let server = serve(None, |_| Reply::ok("hi")).await;
    let mut env = env();
    env.overrides.hosts = [(
        "both.test".to_owned(),
        vec![ip("169.254.169.254"), ip("127.0.0.1")],
    )]
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>()
    .into();
    let url = format!("http://both.test:{}/", server.addr.port());
    let result = probe(&target("x"), &http(&url), None, &env).await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(result.resolved_ip, Some(ip("127.0.0.1")));
}

#[tokio::test]
async fn tcp_and_tls_and_icmp_refuse_refused_hosts_by_literal_and_by_name() {
    let mut env = strict_env();
    env.overrides.hosts = [("loop.test".to_owned(), vec![ip("127.0.0.1")])]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>()
        .into();
    let configs = [
        CheckConfig::Tcp {
            port: 9,
            timeout_ms: 1000,
        },
        CheckConfig::Tls {
            port: 9,
            server_name: None,
            timeout_ms: 1000,
        },
        CheckConfig::Icmp {
            timeout_ms: 1000,
            ip_version: crate::checks::config::IpVersion::V4,
        },
    ];
    for config in &configs {
        for host in [
            "127.0.0.1",
            "169.254.169.254",
            "loop.test",
            "::ffff:127.0.0.1",
        ] {
            let result = probe(&target(host), config, None, &env).await;
            if host == "::ffff:127.0.0.1" && matches!(config, CheckConfig::Icmp { .. }) {
                assert!(!result.ok); // no IPv4 address of that name: refused either way
                continue;
            }
            assert_refused(&result);
        }
    }
}

#[tokio::test]
async fn a_failure_message_never_holds_the_address() {
    let result = probe(
        &target("169.254.169.254"),
        &CheckConfig::Tcp {
            port: 80,
            timeout_ms: 1000,
        },
        None,
        &strict_env(),
    )
    .await;
    let message = result.message.unwrap();
    assert_eq!(message, Failure::NotPermitted.to_string());
    assert!(!message.contains("169"));
}
