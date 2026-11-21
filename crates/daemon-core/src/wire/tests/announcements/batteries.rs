//! The batteries of a batch (K8 Section 4, MET-032): optional, so a batch without them leaves
//! them unchanged, and each battery held to the label, count and range rules.

use super::*;
use crate::{
    readings::{Battery, BatteryState as ReadState},
    wire::{BatchBattery, BatteryState},
};

fn battery(name: &str) -> BatchBattery {
    BatchBattery {
        name: name.to_owned(),
        state: BatteryState::Discharging,
        seconds_left: Some(9000),
        health_percent: Some(91),
    }
}

fn with_batteries(batteries: Vec<BatchBattery>) -> impl FnOnce(&mut Batch) {
    move |batch: &mut Batch| batch.batteries = Some(batteries)
}

#[test]
fn met_032_a_batch_without_batteries_is_taken_and_leaves_them_unchanged() {
    assert!(!BATCH.contains("batteries"));
    assert_eq!(checked(|_| {}).unwrap().batteries, None);
}

#[test]
fn met_032_the_batteries_of_a_batch_reach_the_hub_as_sent_and_an_empty_list_too() {
    let sent = vec![battery("BAT0"), battery("BAT1")];
    assert_eq!(
        checked(with_batteries(sent.clone())).unwrap().batteries,
        Some(sent)
    );
    assert_eq!(
        checked(with_batteries(Vec::new())).unwrap().batteries,
        Some(Vec::new())
    );
}

#[test]
fn met_032_batteries_travel_as_the_k8_example_and_an_unknown_state_reads_as_unknown() {
    let read = Battery {
        name: "BAT0".to_owned(),
        percent: Some(85.0),
        state: ReadState::Discharging,
        seconds_left: Some(9000),
        health_percent: Some(91),
    };
    assert_eq!(
        serde_json::to_string(&BatchBattery::from(&read)).unwrap(),
        r#"{"name":"BAT0","state":"discharging","secondsLeft":9000,"healthPercent":91}"#
    );
    let bare = BatchBattery {
        seconds_left: None,
        health_percent: None,
        state: BatteryState::NotCharging,
        ..battery("BAT0")
    };
    assert_eq!(
        serde_json::to_string(&bare).unwrap(),
        r#"{"name":"BAT0","state":"not_charging"}"#
    );
    let newer: BatchBattery =
        serde_json::from_str(r#"{"name":"BAT0","state":"overheating"}"#).unwrap();
    assert_eq!(newer.state, BatteryState::Unknown);
    assert_eq!(ReadState::from(newer.state), ReadState::Unknown);
}

#[test]
fn met_032_batteries_that_break_the_label_count_or_range_rules_are_malformed() {
    let many = (0..9).map(|i| battery(&format!("BAT{i}"))).collect();
    assert_malformed("9 batteries", with_batteries(many));
    assert_malformed(
        "a repeated name",
        with_batteries(vec![battery("BAT0"), battery("BAT0")]),
    );
    assert_malformed("an empty name", with_batteries(vec![battery("")]));
    assert_malformed(
        "a control character",
        with_batteries(vec![battery("B\u{7}")]),
    );
    assert_malformed(
        "a long name",
        with_batteries(vec![battery(&"x".repeat(129))]),
    );
    let mut late = battery("BAT0");
    late.seconds_left = Some(1 << 31);
    assert_malformed("a time past i32", with_batteries(vec![late]));
    let mut healthy = battery("BAT0");
    healthy.health_percent = Some(101);
    assert_malformed("a health past 100", with_batteries(vec![healthy]));
    let eight = (0..8).map(|i| battery(&format!("BAT{i}"))).collect();
    assert_accepted("8 batteries", with_batteries(eight));
}
