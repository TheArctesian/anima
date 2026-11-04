//! The bind table (NET-003, SEC-008, D-13 with R-021 H-1b).

use std::net::IpAddr;

use super::super::{Error, check_bind};

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn net_003_the_unspecified_address_is_refused_with_or_without_tls() {
    for text in ["0.0.0.0", "::", "::ffff:0.0.0.0"] {
        for tls in [true, false] {
            let refused = check_bind(ip(text), tls);
            assert!(matches!(refused, Err(Error::Unspecified(_))), "{text}");
        }
    }
}

#[test]
fn sec_008_public_and_unlisted_addresses_are_refused() {
    let refused = [
        "8.8.8.8",
        "203.0.113.5",
        "11.0.0.1",
        "9.255.255.255",
        "172.15.255.255",
        "172.32.0.0",
        "192.167.255.255",
        "192.169.0.0",
        "100.63.255.255",
        "100.128.0.0",
        "169.254.1.1",
        "224.0.0.1",
        "255.255.255.255",
        "2001:db8::1",
        "2606:4700::1",
        "fe80::1",
        "fe00::1",
        "ff02::1",
        "::ffff:8.8.8.8",
        "::ffff:100.128.0.1",
    ];
    for text in refused {
        for tls in [true, false] {
            let outcome = check_bind(ip(text), tls);
            assert!(matches!(outcome, Err(Error::Public(_))), "{text}: {outcome:?}");
        }
    }
}

#[test]
fn net_003_loopback_private_vpn_and_unique_local_addresses_are_accepted_with_tls() {
    let accepted = [
        "127.0.0.1",
        "127.255.255.254",
        "::1",
        "::ffff:127.0.0.1",
        "10.0.0.0",
        "10.255.255.255",
        "172.16.0.0",
        "172.30.0.10",
        "172.31.255.255",
        "192.168.0.0",
        "192.168.255.255",
        "100.64.0.0",
        "100.100.1.2",
        "100.127.255.255",
        "::ffff:100.64.0.7",
        "::ffff:10.1.2.3",
        "fc00::",
        "fd7a:115c:a1e0::1",
        "fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
    ];
    for text in accepted {
        let outcome = check_bind(ip(text), true);
        assert!(outcome.is_ok(), "{text}: {outcome:?}");
    }
}

#[test]
fn sec_001_plain_http_is_accepted_only_on_loopback() {
    for text in ["127.0.0.1", "::1", "::ffff:127.0.0.1"] {
        assert!(check_bind(ip(text), false).is_ok(), "{text}");
    }
    for text in ["10.0.0.1", "172.30.0.10", "192.168.1.1", "100.64.0.1", "fd00::1"] {
        let outcome = check_bind(ip(text), false);
        assert!(matches!(outcome, Err(Error::TlsRequired(_))), "{text}");
    }
}

#[test]
fn net_003_the_refusal_names_the_variable_and_the_address() {
    let text = check_bind(ip("::ffff:8.8.8.8"), true).unwrap_err().to_string();
    assert!(text.starts_with("ANIMA_LISTEN=8.8.8.8 "), "{text}");
    let text = check_bind(ip("0.0.0.0"), true).unwrap_err().to_string();
    assert!(text.contains("unspecified"), "{text}");
}
