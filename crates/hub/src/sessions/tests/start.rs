//! The pure checks `start` runs before it writes anything: display bounds (K4 §4, GW-012).
//! The IP literals the gateway must not reach are the address policy's (`targets::tests`).

use proptest::prelude::*;

use crate::sessions::start::{DisplayRequest, checked_display, held_timezone};

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
fn r_032_e_6_a_ticket_holds_only_a_zone_name_of_at_most_64_bytes_else_none() {
    let held = |zone: &str| held_timezone(Some(zone.to_owned()));
    let at_cap = format!("Etc/{}", "x".repeat(60));
    assert_eq!(held(&at_cap), Some(at_cap.clone()));
    assert_eq!(
        held("Pacific/Auckland").as_deref(),
        Some("Pacific/Auckland")
    );
    assert_eq!(held_timezone(None), None);
    for dropped in [
        format!("{at_cap}x"),
        "Europe/../Berlin".to_owned(),
        "Europe/Berlin\n".to_owned(),
        "/etc/localtime".to_owned(),
        "a//b".to_owned(),
        String::new(),
    ] {
        assert_eq!(held(&dropped), None, "{dropped:?}");
    }
}
