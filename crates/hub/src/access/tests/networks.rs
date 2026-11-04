//! Permitted-network rules without a database (NET-002, D-10, D-11): membership for IPv4,
//! IPv6 and IPv4-mapped addresses, how long a list is used, and the refusal-event limit.

use std::net::{IpAddr, Ipv4Addr};

use chrono::TimeDelta;
use ipnet::IpNet;

use super::now;
use crate::access::networks::{
    Loaded, OncePerSpan, Refusals, canonical_net, only_untrusted, permits, untrusted,
};

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

fn nets(texts: &[&str]) -> Vec<IpNet> {
    texts.iter().map(|text| text.parse().unwrap()).collect()
}

#[test]
fn net_002_ipv4_inside_a_network_passes_and_outside_is_refused() {
    let permitted = nets(&["100.64.0.0/10", "192.168.10.0/24"]);
    for inside in [
        "100.64.0.0",
        "100.100.7.7",
        "100.127.255.255",
        "192.168.10.200",
    ] {
        assert!(permits(&permitted, ip(inside)), "{inside}");
    }
    for outside in [
        "100.63.255.255",
        "100.128.0.0",
        "192.168.11.1",
        "8.8.8.8",
        "127.0.0.1",
    ] {
        assert!(!permits(&permitted, ip(outside)), "{outside}");
    }
}

#[test]
fn net_002_ipv6_inside_a_network_passes_and_outside_is_refused() {
    let permitted = nets(&["fd7a:115c:a1e0::/48"]);
    assert!(permits(&permitted, ip("fd7a:115c:a1e0::1")));
    assert!(permits(&permitted, ip("fd7a:115c:a1e0:ffff::9")));
    assert!(!permits(&permitted, ip("fd7a:115c:a1e1::1")));
    assert!(!permits(&permitted, ip("::1")));
}

#[test]
fn d_11_an_ipv4_mapped_address_counts_as_its_ipv4_address() {
    let permitted = nets(&["100.64.0.0/10"]);
    assert!(permits(&permitted, ip("::ffff:100.64.0.7")));
    assert!(!permits(&permitted, ip("::ffff:8.8.8.8")));
    let mapped_net = nets(&["::ffff:10.0.0.0/104"]);
    assert!(permits(&mapped_net, ip("10.1.2.3")));
    assert!(permits(&mapped_net, ip("::ffff:10.1.2.3")));
    assert!(!permits(&mapped_net, ip("11.0.0.1")));
}

#[test]
fn d_11_an_ipv4_client_is_never_inside_an_ipv6_network() {
    let everything_v6 = nets(&["::/0"]);
    assert!(permits(&everything_v6, ip("2001:db8::1")));
    assert!(!permits(&everything_v6, ip("10.1.2.3")));
    assert!(!permits(&everything_v6, ip("::ffff:10.1.2.3")));
}

#[test]
fn h_2_container_and_proxy_addresses_are_untrusted() {
    for addr in [
        "172.16.0.0",
        "172.30.0.1",
        "172.31.255.255",
        "::ffff:172.30.0.10",
    ] {
        assert!(untrusted(ip(addr)), "{addr}");
    }
    for addr in [
        "172.15.255.255",
        "172.32.0.0",
        "100.64.0.7",
        "10.0.0.1",
        "fd00::172",
    ] {
        assert!(!untrusted(ip(addr)), "{addr}");
    }
    for net in ["172.30.0.0/24", "172.16.0.0/12", "::ffff:172.20.0.0/112"] {
        assert!(only_untrusted(net.parse().unwrap()), "{net}");
    }
    for net in ["172.0.0.0/8", "0.0.0.0/0", "172.32.0.0/16", "100.64.0.0/10"] {
        assert!(!only_untrusted(net.parse().unwrap()), "{net}");
    }
}

#[test]
fn net_002_an_empty_list_permits_nothing() {
    assert!(!permits(&[], ip("100.64.0.1")));
    assert!(!permits(&[], ip("::1")));
}

#[test]
fn d_10_networks_lose_host_bits_and_mapped_networks_become_ipv4() {
    let cases = [
        ("10.1.2.3/8", "10.0.0.0/8"),
        ("fd00::1/8", "fd00::/8"),
        ("::ffff:10.0.0.0/104", "10.0.0.0/8"),
        ("::ffff:1.2.3.4/128", "1.2.3.4/32"),
        ("::ffff:0.0.0.0/96", "0.0.0.0/0"),
        ("::ffff:0:0/95", "::fffe:0:0/95"),
        ("::/0", "::/0"),
    ];
    for (given, expected) in cases {
        let given: IpNet = given.parse().unwrap();
        assert_eq!(canonical_net(given), expected.parse().unwrap(), "{given}");
    }
}

#[test]
fn d_10_a_loaded_list_is_used_for_30_seconds() {
    let loaded = Loaded::new(nets(&["10.1.2.3/8"]), now());
    assert_eq!(&*loaded.networks, &nets(&["10.0.0.0/8"])[..]);
    assert!(loaded.fresh_at(now()));
    assert!(loaded.fresh_at(now() + TimeDelta::seconds(29)));
    assert!(!loaded.fresh_at(now() + TimeDelta::seconds(30)));
    assert!(
        !loaded.fresh_at(now() - TimeDelta::seconds(1)),
        "clock went back"
    );
}

#[test]
fn d_11_one_refusal_event_per_address_per_minute() {
    let mut refusals = Refusals::default();
    let scanner = ip("203.0.113.9");
    assert!(refusals.admit(scanner, now()));
    assert!(!refusals.admit(scanner, now()));
    assert!(!refusals.admit(scanner, now() + TimeDelta::seconds(59)));
    assert!(refusals.admit(scanner, now() + TimeDelta::seconds(60)));
    assert!(refusals.admit(ip("203.0.113.10"), now() + TimeDelta::seconds(60)));
    assert!(
        refusals.admit(scanner, now() - TimeDelta::hours(1)),
        "a clock that went back forgets newer entries"
    );
}

#[test]
fn d_11_a_scan_from_many_addresses_writes_at_most_30_events_a_minute() {
    let mut refusals = Refusals::default();
    let written = (0..1000_u32)
        .filter(|n| refusals.admit(IpAddr::V4(Ipv4Addr::from(0xcb00_7100 + n)), now()))
        .count();
    assert_eq!(written, 30);
    assert_eq!(refusals.len(), 30, "memory stays bounded");
    let later = now() + TimeDelta::minutes(1);
    assert!(refusals.admit(ip("198.51.100.1"), later));
    assert_eq!(refusals.len(), 1, "a minute later the old entries are gone");
}

#[test]
fn d_11_refusals_the_budget_left_out_are_counted_once_a_minute() {
    let scan = |refusals: &mut Refusals, from: u32, at| {
        for n in from..from + 100 {
            refusals.admit(IpAddr::V4(Ipv4Addr::from(0xcb00_7100 + n)), at);
        }
    };
    let mut refusals = Refusals::default();
    assert_eq!(refusals.unrecorded(now()), None, "nothing left out yet");
    scan(&mut refusals, 0, now());
    assert_eq!(
        refusals.unrecorded(now()),
        None,
        "not before a minute has passed"
    );
    let second = now() + TimeDelta::seconds(59);
    assert!(
        !refusals.admit(ip("198.18.0.9"), now()),
        "the budget is spent"
    );
    assert!(!refusals.admit(ip("198.18.0.9"), second));
    assert_eq!(refusals.unrecorded(second), None);
    let minute = now() + TimeDelta::minutes(1);
    assert_eq!(
        refusals.unrecorded(minute),
        Some(72),
        "70 of the scan and 2 more"
    );
    assert_eq!(refusals.unrecorded(minute), None, "counted once");
    let repeat = ip("198.51.100.7");
    assert!(refusals.admit(repeat, minute));
    assert!(!refusals.admit(repeat, minute));
    assert_eq!(
        refusals.unrecorded(minute + TimeDelta::minutes(5)),
        None,
        "a repeat within the minute is not a left-out refusal"
    );
    scan(&mut refusals, 200, minute);
    let back = minute - TimeDelta::hours(1);
    assert_eq!(refusals.unrecorded(back), Some(71), "clock went back");
}

#[test]
fn net_002_a_list_that_cannot_be_read_is_logged_once_a_minute() {
    let mut logged = OncePerSpan::default();
    assert!(logged.due(now()));
    assert!(!logged.due(now()));
    assert!(!logged.due(now() + TimeDelta::seconds(59)));
    assert!(logged.due(now() + TimeDelta::seconds(60)));
    assert!(logged.due(now() - TimeDelta::hours(1)), "clock went back");
}
