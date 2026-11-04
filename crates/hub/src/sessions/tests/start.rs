//! The pure checks `start` runs before it writes anything: display bounds (K4 §4, GW-012)
//! and IP literals the gateway must not reach.

use std::net::IpAddr;

use proptest::prelude::*;

use crate::sessions::start::{DisplayRequest, checked_display, is_refused_target};

fn request(width_px: i64, height_px: i64, dpi: i64) -> DisplayRequest {
    DisplayRequest {
        width_px,
        height_px,
        dpi,
        timezone: None,
    }
}

proptest! {
    /// Accepted exactly when width and height are 1..=8192 and DPI 48..=480; otherwise each
    /// field outside its bounds is named once, in input order.
    #[test]
    fn gw_012_display_is_accepted_only_within_bounds(
        width in -10_i64..9000,
        height in -10_i64..9000,
        dpi in -10_i64..600,
    ) {
        let side_ok = |v: i64| (1..=8192).contains(&v);
        let dpi_ok = (48..=480).contains(&dpi);
        match checked_display(&request(width, height, dpi)) {
            Ok(display) => {
                prop_assert!(side_ok(width) && side_ok(height) && dpi_ok);
                prop_assert_eq!(
                    (i64::from(display.width), i64::from(display.height), i64::from(display.dpi)),
                    (width, height, dpi)
                );
                prop_assert!(display.within_bounds());
            }
            Err(fields) => {
                let expected: Vec<&str> = [
                    (!side_ok(width)).then_some("display.widthPx"),
                    (!side_ok(height)).then_some("display.heightPx"),
                    (!dpi_ok).then_some("display.dpi"),
                ]
                .into_iter()
                .flatten()
                .collect();
                prop_assert!(!expected.is_empty());
                let named: Vec<&str> = fields.iter().map(|field| field.field).collect();
                prop_assert_eq!(named, expected);
            }
        }
    }
}

#[test]
fn gw_012_values_beyond_u32_are_refused_not_wrapped() {
    let huge = i64::from(u32::MAX) + 1_281;
    assert!(checked_display(&request(huge, 800, 96)).is_err());
    assert!(checked_display(&request(1280, 800, i64::MIN)).is_err());
}

#[test]
fn gw_011_loopback_link_local_multicast_and_unspecified_literals_are_refused() {
    for refused in [
        "127.0.0.1",
        "127.255.255.254",
        "169.254.0.1",
        "224.0.0.1",
        "239.255.255.250",
        "255.255.255.255",
        "0.0.0.0",
        "0.1.2.3",
        "::1",
        "::",
        "fe80::1",
        "febf::1",
        "ff02::1",
        "::ffff:127.0.0.1",
        "::ffff:169.254.169.254",
        // IPv4-compatible ::/96, SIIT ::ffff:0:0:0/96 and NAT64 64:ff9b::/96.
        "::127.0.0.1",
        "::169.254.169.254",
        "::224.0.0.1",
        "::ffff:0:127.0.0.1",
        "::ffff:0:169.254.1.1",
        "64:ff9b::127.0.0.1",
        "64:ff9b::a9fe:a9fe",
        "64:ff9b::255.255.255.255",
    ] {
        assert!(
            is_refused_target(refused.parse::<IpAddr>().unwrap()),
            "{refused}"
        );
    }
    for allowed in [
        "10.10.0.2",
        "192.168.1.10",
        "100.64.0.7",
        "1.1.1.1",
        "fd7a:115c:a1e0::1",
        "2001:db8::1",
        "fec0::1",
        "::ffff:10.10.0.2",
        "::10.10.0.2",
        "::ffff:0:10.10.0.2",
        "64:ff9b::10.10.0.2",
        // Outside the embedding prefixes the last 32 bits mean nothing.
        "2001:db8::127.0.0.1",
        "64:ff9b:1::127.0.0.1",
    ] {
        assert!(
            !is_refused_target(allowed.parse::<IpAddr>().unwrap()),
            "{allowed}"
        );
    }
}
