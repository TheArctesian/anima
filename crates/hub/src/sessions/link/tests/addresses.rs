//! D-1, R-032, P2-004: the deny list refuses every address the strict target rule refuses, so
//! a test can reach loopback with `AddressPolicy::allowing_loopback` alone; and one refused
//! address refuses the whole host.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use proptest::prelude::*;

use crate::{
    sessions::{link::connect::permitted, target::is_refused_target},
    targets::AddressPolicy,
};

/// The strict rule refuses `ip`, and so does the default policy (and one with an extra deny,
/// which only refuses more).
fn refused_by_both(ip: IpAddr) -> Result<(), TestCaseError> {
    if is_refused_target(ip) {
        prop_assert!(
            !AddressPolicy::default().permits(ip),
            "{ip} passes the policy"
        );
        let extra = AddressPolicy::new(vec!["10.0.0.0/8".parse().unwrap()]);
        prop_assert!(
            !extra.permits(ip),
            "{ip} passes a policy with an extra deny"
        );
    }
    Ok(())
}

/// An IPv6 address made of `prefix` (the first `bits` bits) and random `rest`.
fn under(prefix: u128, bits: u32, rest: u128) -> IpAddr {
    let mask = u128::MAX.checked_shr(bits).unwrap_or(0);
    IpAddr::V6(Ipv6Addr::from((prefix & !mask) | (rest & mask)))
}

/// The IPv6 prefixes the strict rule reads an IPv4 address in, or refuses outright, with
/// their lengths.
const PREFIXES: [(u128, u32); 10] = [
    (0, 96),                      // IPv4-compatible
    (0xffff << 32, 96),           // IPv4-mapped
    (0xffff << 48, 96),           // SIIT
    (0x0064_ff9b << 96, 96),      // NAT64
    (0x0064_ff9b_0001 << 80, 48), // local-use NAT64
    (0x2002 << 112, 16),          // 6to4
    (0x2001 << 112, 32),          // Teredo
    (0xfe80 << 112, 10),          // link-local
    (0xff00 << 112, 8),           // multicast
    (1, 128),                     // loopback
];

/// IPv4 addresses that sit in a refused range, as the embedded part of the prefixes above.
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
    fn d_1_the_policy_refuses_every_ipv4_address_the_rule_refuses(raw in any::<u32>()) {
        refused_by_both(IpAddr::V4(Ipv4Addr::from(raw)))?;
        for v4 in refused_v4(raw) {
            refused_by_both(IpAddr::V4(Ipv4Addr::from(v4)))?;
        }
    }

    #[test]
    fn d_1_the_policy_refuses_every_ipv6_address_the_rule_refuses(raw in any::<u128>(), v4 in any::<u32>()) {
        refused_by_both(IpAddr::V6(Ipv6Addr::from(raw)))?;
        for (prefix, bits) in PREFIXES {
            refused_by_both(under(prefix, bits, raw))?;
            for embedded in refused_v4(v4) {
                // The IPv4 address in the last 32 bits, and in bits 16 to 47 for 6to4.
                refused_by_both(under(prefix, bits.min(96), u128::from(embedded)))?;
                refused_by_both(under(prefix, bits, u128::from(embedded) << 80))?;
            }
        }
    }
}

#[test]
fn d_1_the_edges_of_each_refused_range_are_refused_by_the_policy() {
    let edges = [
        "127.0.0.1",
        "127.255.255.255",
        "0.0.0.0",
        "0.255.255.255",
        "169.254.0.0",
        "169.254.169.254",
        "224.0.0.1",
        "239.255.255.255",
        "255.255.255.255",
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "::127.0.0.1",
        "::ffff:0:127.0.0.1",
        "64:ff9b::7f00:1",
        "64:ff9b:1::1",
        "2002:7f00:1::",
        "2002:a9fe:a9fe::",
        "2001::1",
        "fe80::1",
        "febf::1",
        "ff02::1",
    ];
    for edge in edges {
        let ip: IpAddr = edge.parse().unwrap();
        assert!(is_refused_target(ip), "{edge} is a refused edge");
        assert!(!AddressPolicy::default().permits(ip), "{edge}");
        assert!(
            !AddressPolicy::default().permits(ip.to_canonical()),
            "{edge}"
        );
    }
}

fn socket(text: &str) -> SocketAddr {
    SocketAddr::new(text.parse().unwrap(), 22)
}

#[test]
fn p2_004_a_host_whose_addresses_all_pass_keeps_them_all_in_order() {
    let addresses = vec![socket("10.10.0.21"), socket("fd00::21")];
    assert_eq!(
        permitted(&AddressPolicy::default(), addresses.clone()),
        Some(addresses)
    );
}

#[test]
fn d_1_one_refused_address_refuses_the_host_whichever_comes_first() {
    let policy = AddressPolicy::new(vec!["10.10.0.0/24".parse().unwrap()]);
    for addresses in [
        vec![socket("10.20.0.5"), socket("10.10.0.9")],
        vec![socket("10.10.0.9"), socket("10.20.0.5")],
        vec![socket("10.20.0.5"), socket("169.254.169.254")],
    ] {
        assert_eq!(permitted(&policy, addresses.clone()), None, "{addresses:?}");
    }
}

#[test]
fn d_1_the_default_policy_refuses_loopback_and_only_the_test_policy_reaches_it() {
    let loopback = vec![socket("127.0.0.1")];
    assert_eq!(permitted(&AddressPolicy::default(), loopback.clone()), None);
    let denied = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]).allowing_loopback();
    assert_eq!(
        permitted(&denied, loopback.clone()),
        None,
        "extra deny wins"
    );
    assert_eq!(
        permitted(
            &AddressPolicy::default().allowing_loopback(),
            loopback.clone()
        ),
        Some(loopback)
    );
}
