//! The probe's edges (R-032, D-5, R-034): refused targets, a closed port, a host that never
//! answers, one that does not speak SSH, and a key read with no authentication. The sign-in
//! runs against the recording server in `tests/it/sessions/host_keys`.

use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, LazyLock},
    time::Duration,
};

use russh::{
    keys::{PrivateKey, ssh_key::private::Ed25519Keypair},
    server,
};
use tokio::{io::AsyncWriteExt, net::TcpListener};

use crate::{
    sessions::{
        probe::read_host_key,
        public_key::PublicKey,
        ssh::{SshError, Target},
        target::TargetRule,
    },
    targets::AddressPolicy,
};

/// A server that only presents its key; every authentication is refused by default.
struct KeyOnly;

impl server::Handler for KeyOnly {
    type Error = russh::Error;
}

#[tokio::test]
async fn r_034_the_probe_returns_the_key_the_host_presents() {
    let host_key = PrivateKey::from(Ed25519Keypair::from_seed(&[3; 32]));
    let blob = host_key.public_key().to_bytes().unwrap();
    let config = Arc::new(server::Config {
        keys: vec![host_key],
        ..server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let serve = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        if let Ok(session) = server::run_stream(config, stream, KeyOnly).await {
            let _ended = session.await;
        }
    });

    let read = read_host_key(loopback(port), Duration::from_secs(5)).await;

    assert_eq!(read, Ok(PublicKey::from_blob(blob).unwrap()));
    serve.abort();
}

const SHORT: Duration = Duration::from_millis(300);

/// The hub's rule with no deny list, as `Live::default()` holds it.
static STRICT: LazyLock<TargetRule> = LazyLock::new(TargetRule::default);
/// The tests' rule, which reaches loopback.
static LOOPBACK_FOR_TESTS: LazyLock<TargetRule> =
    LazyLock::new(|| TargetRule::default().reaching_loopback_for_tests());

fn strict(host: &str, port: u16) -> Target<'_> {
    Target {
        host,
        port,
        rule: &STRICT,
    }
}

fn loopback(port: u16) -> Target<'static> {
    Target {
        host: "127.0.0.1",
        port,
        rule: &LOOPBACK_FOR_TESTS,
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
            Err(SshError::RefusedTarget),
            "{host}"
        );
    }
    for host in ["169.254.169.254", "fe80::1", "224.0.0.1", "64:ff9b::7f00:1"] {
        assert_eq!(
            read_host_key(strict(host, 22), SHORT).await,
            Err(SshError::RefusedTarget),
            "{host}"
        );
    }
    let accepted = tokio::time::timeout(Duration::from_millis(50), listener.accept()).await;
    assert!(accepted.is_err(), "the probe never connected");
}

#[test]
fn r_032_the_test_rule_lets_only_plain_loopback_through() {
    let rule = &*LOOPBACK_FOR_TESTS;
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
        assert!(STRICT.refuses(ip), "{refused}");
    }
}

#[test]
fn r_032_a_name_is_refused_when_any_of_its_addresses_is() {
    let addresses = |list: &[&str]| -> Vec<SocketAddr> {
        list.iter()
            .map(|ip| SocketAddr::new(ip.parse().unwrap(), 22))
            .collect()
    };
    let public = addresses(&["10.10.0.2", "2001:db8::1"]);
    assert_eq!(STRICT.checked(public.clone()), Some(public));
    for mixed in [
        addresses(&["10.10.0.2", "127.0.0.1"]),
        addresses(&["::1", "10.10.0.2"]),
        addresses(&["10.10.0.2", "169.254.169.254", "10.10.0.3"]),
    ] {
        assert_eq!(STRICT.checked(mixed.clone()), None, "{mixed:?}");
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
        Err(SshError::Unreachable)
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
        Err(SshError::Timeout)
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
        Err(SshError::Protocol)
    );
    http.await.unwrap();
}

fn refused(text: &str) -> bool {
    STRICT.refuses(text.parse::<IpAddr>().unwrap())
}

#[test]
fn r_032_a_6to4_address_is_refused_when_the_ipv4_it_carries_is() {
    for carried_refused in [
        "2002:7f00:1::1",
        "2002:a9fe:a9fe::",
        "2002:e000:1::5",
        "2002::1",
    ] {
        assert!(refused(carried_refused), "{carried_refused}");
    }
    for carried_allowed in ["2002:a0a:2::1", "2002:c0a8:10a::7f00:1"] {
        assert!(!refused(carried_allowed), "{carried_allowed}");
    }
}

#[test]
fn r_032_a_teredo_address_is_refused_outright() {
    for teredo in [
        "2001::1",
        "2001:0:4136:e378:8000:63bf:3fff:fdd2",
        "2001:0:a0a:2::",
    ] {
        assert!(refused(teredo), "{teredo}");
    }
    for outside in ["2001:1::1", "2001:db8::1", "2001:470::1"] {
        assert!(!refused(outside), "{outside}");
    }
}

#[test]
fn r_032_a_local_use_nat64_address_is_refused_outright() {
    for local in ["64:ff9b:1::a0a:2", "64:ff9b:1:ffff::1", "64:ff9b:1::"] {
        assert!(refused(local), "{local}");
    }
    for outside in ["64:ff9b::a0a:2", "64:ff9b:2::1"] {
        assert!(!refused(outside), "{outside}");
    }
}

/// The default rule with `cidr` denied, as `main.rs` builds it from `ANIMA_PROBE_DENY`.
fn denying(cidr: &str) -> TargetRule {
    TargetRule::default().denying(AddressPolicy::new(vec![cidr.parse().unwrap()]))
}

#[test]
fn r_030_the_deny_list_refuses_its_networks_in_every_ipv6_form() {
    let rule = denying("10.20.0.0/16");
    for denied in [
        "10.20.1.5",
        "::ffff:10.20.1.5",
        "::ffff:0:10.20.1.5",
        "::10.20.1.5",
        "64:ff9b::10.20.1.5",
        "2002:a14:105::1",
    ] {
        assert!(rule.refuses(denied.parse().unwrap()), "{denied}");
    }
    for allowed in ["10.10.0.2", "::ffff:10.10.0.2", "192.168.1.10"] {
        assert!(!rule.refuses(allowed.parse().unwrap()), "{allowed}");
    }
    let addresses = vec![
        SocketAddr::new("10.10.0.2".parse().unwrap(), 22),
        SocketAddr::new("10.20.1.5".parse().unwrap(), 22),
    ];
    assert_eq!(
        rule.checked(addresses),
        None,
        "any denied address refuses the name"
    );
}

#[test]
fn r_030_the_default_rule_keeps_device_ranges_and_refuses_the_policys_own() {
    for device in [
        "10.20.1.5",
        "172.16.0.1",
        "192.168.1.10",
        "100.64.0.7",
        "fd7a:115c:a1e0::1",
    ] {
        assert!(!refused(device), "{device}");
    }
    // Tailscale's own resolver, the Azure wire server, the AWS IPv6 metadata address and the
    // IETF protocol and reserved ranges host no device.
    for never in [
        "100.100.100.100",
        "168.63.129.16",
        "fd00:ec2::254",
        "192.0.0.1",
        "240.0.0.1",
    ] {
        assert!(refused(never), "{never}");
    }
}

#[test]
fn r_030_the_loopback_test_rule_still_obeys_the_deny_list_in_either_order() {
    let deny = || AddressPolicy::new(vec!["127.0.0.2/32".parse().unwrap()]);
    let before = TargetRule::default()
        .denying(deny())
        .reaching_loopback_for_tests();
    let after = TargetRule::default()
        .reaching_loopback_for_tests()
        .denying(deny());
    for rule in [before, after] {
        assert!(rule.refuses("127.0.0.2".parse().unwrap()));
        assert!(!rule.refuses("127.0.0.1".parse().unwrap()));
    }
}

#[tokio::test]
async fn r_030_the_probe_never_connects_to_a_denied_address() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let rule = TargetRule::default()
        .reaching_loopback_for_tests()
        .denying(AddressPolicy::new(vec!["127.0.0.1/32".parse().unwrap()]));
    for host in ["127.0.0.1", "localhost", "::ffff:127.0.0.1"] {
        let target = Target {
            host,
            port,
            rule: &rule,
        };
        let read = read_host_key(target, SHORT).await;
        assert_eq!(read, Err(SshError::RefusedTarget), "{host}");
    }
    let accepted = tokio::time::timeout(Duration::from_millis(50), listener.accept()).await;
    assert!(accepted.is_err(), "the probe never connected");
}
