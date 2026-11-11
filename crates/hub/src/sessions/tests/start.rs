//! The pure checks `start` runs before it writes anything: display bounds (K4 §4, GW-012) and
//! whether the session is recorded (K4 §1 step 2). The IP literals the gateway must not reach
//! are the address policy's (`targets::tests`).

use std::io::ErrorKind;

use async_graphql::Value;
use proptest::prelude::*;

use crate::{
    events::process_log::tests::capture::capture_log,
    ids::DeviceId,
    sessions::{
        Error, Recording,
        graphql::error::to_graphql,
        recording::CannotRecord,
        start::{DisplayRequest, checked_display, held_timezone, recorded},
    },
};

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

/// How `recorded` asks whether the hub can record.
type Can = fn(&Recording) -> Result<(), CannotRecord>;

/// A recording root whose name a leak would show.
const ROOT: &str = "/srv/anima/recordings-root-7f3a";

const CANNOT_RECORD: &str = "Recording is on for this device, but the hub cannot record now.";

fn storage() -> Recording {
    Recording {
        root: Some(ROOT.into()),
        max_bytes: 1 << 30,
    }
}

fn device() -> DeviceId {
    "0190a000-0000-7000-8002-000000000001".parse().unwrap()
}

#[test]
fn ra_040_a_device_that_does_not_record_is_never_checked() {
    let never: Can = |_| panic!("a device that does not record was checked");
    assert!(matches!(
        recorded(device(), false, &storage(), never),
        Ok(false)
    ));
}

#[test]
fn ra_040_a_device_that_records_is_recorded_when_the_hub_can_record() {
    let can: Can = |recording| {
        assert_eq!(recording.root.as_deref(), Some(ROOT.as_ref()));
        Ok(())
    };
    assert!(matches!(
        recorded(device(), true, &storage(), can),
        Ok(true)
    ));
}

#[test]
fn d_14_each_reason_the_hub_cannot_record_refuses_unavailable_with_k4_text_and_no_path() {
    let (capture, _guard) = capture_log("trace");
    let reasons: [Can; 5] = [
        |_| Err(CannotRecord::Unset),
        |_| Err(CannotRecord::Unreadable(ErrorKind::PermissionDenied)),
        |_| Err(CannotRecord::NotFolder),
        |_| Err(CannotRecord::NotWritable),
        |_| Err(CannotRecord::LowSpace),
    ];
    for can in reasons {
        let error = recorded(device(), true, &storage(), can).unwrap_err();
        assert!(matches!(error, Error::CannotRecord), "{error:?}");
        let graphql = to_graphql(error);
        assert_eq!(graphql.message, CANNOT_RECORD);
        let code = graphql
            .extensions
            .as_ref()
            .and_then(|values| values.get("code"));
        assert_eq!(code, Some(&Value::from("UNAVAILABLE")));
    }
    let log = capture.text();
    assert!(log.contains("the hub cannot record now"), "{log}");
    assert!(
        log.contains("less than 1 GiB free"),
        "the reason is logged: {log}"
    );
    assert!(!log.contains("recordings-root-7f3a"), "LOG-007: {log}");
}
