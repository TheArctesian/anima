//! The one target rule (Q2-09, R-030, R-032) and the checked resolver (P3-003). The cases of
//! the deleted sessions rule (`is_refused_target`) are kept here, aimed at the policy.

mod resolver;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use proptest::prelude::*;

use super::AddressPolicy;

fn refused(text: &str) -> bool {
    !AddressPolicy::default().permits(text.parse().unwrap())
}

/// Every class the deleted sessions rule refused, by name, with its edges.
const REFUSED_CLASSES: &[(&str, &[&str])] = &[
    (
        "loopback",
        &["127.0.0.1", "127.255.255.254", "127.255.255.255", "::1"],
    ),
    (
        "link-local",
        &[
            "169.254.0.0",
            "169.254.0.1",
            "169.254.169.254",
            "fe80::1",
            "febf::1",
        ],
    ),
    (
        "multicast",
        &["224.0.0.1", "239.255.255.250", "239.255.255.255", "ff02::1"],
    ),
    ("broadcast", &["255.255.255.255"]),
    ("this network 0/8", &["0.1.2.3", "0.255.255.255"]),
    ("unspecified", &["0.0.0.0", "::"]),
    (
        "Teredo",
        &[
            "2001::1",
            "2001:0:4136:e378:8000:63bf:3fff:fdd2",
            "2001:0:a0a:2::",
        ],
    ),
    (
        "local-use NAT64",
        &[
            "64:ff9b:1::1",
            "64:ff9b:1::a0a:2",
            "64:ff9b:1:ffff::1",
            "64:ff9b:1::",
        ],
    ),
    (
        "6to4",
        &[
            "2002:7f00:1::",
            "2002:7f00:1::1",
            "2002:a9fe:a9fe::",
            "2002:e000:1::5",
            "2002::1",
        ],
    ),
    (
        "IPv4-mapped",
        &["::ffff:127.0.0.1", "::ffff:169.254.169.254"],
    ),
    (
        "IPv4-compatible",
        &["::127.0.0.1", "::169.254.169.254", "::224.0.0.1"],
    ),
    ("SIIT", &["::ffff:0:127.0.0.1", "::ffff:0:169.254.1.1"]),
    (
        "NAT64",
        &[
            "64:ff9b::7f00:1",
            "64:ff9b::127.0.0.1",
            "64:ff9b::a9fe:a9fe",
            "64:ff9b::255.255.255.255",
        ],
    ),
];

#[test]
fn q2_09_the_policy_refuses_every_class_the_sessions_rule_refused() {
    for (class, addresses) in REFUSED_CLASSES {
        for text in *addresses {
            let ip: IpAddr = text.parse().unwrap();
            assert!(refused(text), "{class}: {text}");
            assert!(
                !AddressPolicy::default().permits(ip.to_canonical()),
                "{class}: {text}"
            );
            let extra = AddressPolicy::new(vec!["10.0.0.0/8".parse().unwrap()]);
            assert!(!extra.permits(ip), "{class}: {text} with an extra deny");
        }
    }
}

#[test]
fn gw_011_device_addresses_and_their_ipv6_forms_pass() {
    for allowed in [
        "10.10.0.2",
        "192.168.1.10",
        "100.64.0.7",
        "1.1.1.1",
        "fd7a:115c:a1e0::1",
        "2001:db8::1",
        "::ffff:10.10.0.2",
        "::10.10.0.2",
        "::ffff:0:10.10.0.2",
        "64:ff9b::10.10.0.2",
        // Outside the embedding prefixes the last 32 bits mean nothing.
        "2001:db8::127.0.0.1",
        // 6to4 carrying an address the hub may reach.
        "2002:a0a:2::1",
        "2002:c0a8:10a::7f00:1",
        // Just outside Teredo and local-use NAT64.
        "2001:1::1",
        "2001:470::1",
        "64:ff9b:2::1",
    ] {
        assert!(!refused(allowed), "{allowed}");
    }
}

#[test]
fn r_030_the_deny_list_refuses_its_networks_in_every_ipv6_form() {
    let policy = AddressPolicy::new(vec!["10.20.0.0/16".parse().unwrap()]);
    for denied in [
        "10.20.1.5",
        "::ffff:10.20.1.5",
        "::ffff:0:10.20.1.5",
        "::10.20.1.5",
        "64:ff9b::10.20.1.5",
        "2002:a14:105::1",
    ] {
        assert!(!policy.permits(denied.parse().unwrap()), "{denied}");
    }
    for allowed in ["10.10.0.2", "::ffff:10.10.0.2", "192.168.1.10"] {
        assert!(policy.permits(allowed.parse().unwrap()), "{allowed}");
    }
}

#[test]
fn r_030_the_default_policy_keeps_device_ranges_and_refuses_its_own() {
    for device in [
        "10.20.1.5",
        "172.16.0.1",
        "192.168.1.10",
        "100.64.0.7",
        "fd7a:115c:a1e0::1",
    ] {
        assert!(!refused(device), "{device}");
    }
    // Tailscale's own resolver, the Azure wire server, the AWS IPv6 metadata address, the
    // IETF protocol and reserved ranges and site-local host no device.
    for never in [
        "100.100.100.100",
        "168.63.129.16",
        "fd00:ec2::254",
        "192.0.0.1",
        "240.0.0.1",
        "fec0::1",
    ] {
        assert!(refused(never), "{never}");
    }
}

#[test]
fn r_032_the_loopback_test_switch_lets_only_plain_loopback_through() {
    let policy = AddressPolicy::default().allowing_loopback();
    for allowed in ["127.0.0.1", "127.0.0.2", "::1", "10.10.0.2"] {
        assert!(policy.permits(allowed.parse().unwrap()), "{allowed}");
    }
    for refused in [
        "::ffff:127.0.0.1",
        "::127.0.0.1",
        "2002:7f00:1::",
        "169.254.1.1",
        "fe80::1",
        "0.0.0.0",
        "ff02::1",
    ] {
        assert!(!policy.permits(refused.parse().unwrap()), "{refused}");
    }
    let denied = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]).allowing_loopback();
    assert!(
        !denied.permits("127.0.0.1".parse().unwrap()),
        "the deny list wins"
    );
}

/// The policy refuses `ip`, with no extra deny and with one (which only refuses more).
fn refused_by_every_policy(ip: IpAddr) -> Result<(), TestCaseError> {
    prop_assert!(
        !AddressPolicy::default().permits(ip),
        "{ip} passes the policy"
    );
    let extra = AddressPolicy::new(vec!["10.0.0.0/8".parse().unwrap()]);
    prop_assert!(
        !extra.permits(ip),
        "{ip} passes a policy with an extra deny"
    );
    Ok(())
}

/// An IPv6 address made of `prefix` (the first `bits` bits) and random `rest`.
fn under(prefix: u128, bits: u32, rest: u128) -> IpAddr {
    let mask = u128::MAX.checked_shr(bits).unwrap_or(0);
    IpAddr::V6(Ipv6Addr::from((prefix & !mask) | (rest & mask)))
}

/// IPv6 prefixes refused whatever follows them, with their lengths.
const REFUSED_OUTRIGHT: [(u128, u32); 5] = [
    (0x0064_ff9b_0001 << 80, 48), // local-use NAT64
    (0x2001 << 112, 32),          // Teredo
    (0xfe80 << 112, 10),          // link-local
    (0xff00 << 112, 8),           // multicast
    (1, 128),                     // loopback
];

/// IPv6 prefixes that carry an IPv4 address in their last 32 bits, with their lengths.
const EMBEDDING: [(u128, u32); 4] = [
    (0, 96),                 // IPv4-compatible
    (0xffff << 32, 96),      // IPv4-mapped
    (0xffff << 48, 96),      // SIIT
    (0x0064_ff9b << 96, 96), // NAT64
];

/// IPv4 addresses that sit in a refused range: loopback, 0/8, link-local, multicast and
/// broadcast.
fn refused_v4(rest: u32) -> [u32; 5] {
    let low = rest & 0x00ff_ffff;
    [
        (127 << 24) | low,
        low,
        (169 << 24) | (254 << 16) | (rest & 0xffff),
        (224 << 24) | (rest & 0x0fff_ffff),
        u32::MAX,
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    #[test]
    fn d_1_every_refused_ipv4_address_is_refused(raw in any::<u32>()) {
        for v4 in refused_v4(raw) {
            refused_by_every_policy(IpAddr::V4(Ipv4Addr::from(v4)))?;
        }
    }

    #[test]
    fn d_1_every_ipv6_form_of_a_refused_address_is_refused(raw in any::<u128>(), v4 in any::<u32>()) {
        for (prefix, bits) in REFUSED_OUTRIGHT {
            refused_by_every_policy(under(prefix, bits, raw))?;
        }
        for embedded in refused_v4(v4) {
            for (prefix, bits) in EMBEDDING {
                refused_by_every_policy(under(prefix, bits, u128::from(embedded)))?;
            }
            // 6to4 carries the IPv4 address in bits 16 to 47.
            refused_by_every_policy(under(0x2002 << 112, 16, (u128::from(embedded) << 80) | (raw & ((1 << 80) - 1))))?;
        }
    }
}
