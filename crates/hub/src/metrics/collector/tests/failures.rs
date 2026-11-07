//! One event when a device starts failing and one when it works again (D-10, LOG-007).

use crate::{
    events::{Severity, Source},
    ids::DeviceId,
    metrics::collector::failures::{Change, FAILING_MESSAGE, WORKING_MESSAGE, change, event},
};

#[test]
fn d10_only_a_change_of_outcome_is_reported() {
    assert_eq!(change(false, false), Some(Change::StartedFailing));
    assert_eq!(change(true, true), Some(Change::WorksAgain));
    assert_eq!(change(true, false), None, "a repeat failure");
    assert_eq!(change(false, true), None, "a repeat success");
}

#[test]
fn d10_failing_is_a_warning_and_working_again_is_info_from_the_device() {
    let device: DeviceId = "0190a000-0000-7000-8002-000000000002".parse().unwrap();
    let failing = event(Change::StartedFailing, device, "docker-host".to_owned());
    assert_eq!(failing.severity, Severity::Warning);
    assert_eq!(failing.source, Source::Device);
    assert_eq!(failing.device, Some((device, "docker-host".to_owned())));
    assert_eq!(failing.message, FAILING_MESSAGE);
    let working = event(Change::WorksAgain, device, "docker-host".to_owned());
    assert_eq!(working.severity, Severity::Info);
    assert_eq!(working.source, Source::Device);
    assert_eq!(working.message, WORKING_MESSAGE);
}

#[test]
fn log_007_the_messages_are_fixed_text_without_the_device() {
    let device: DeviceId = "0190a000-0000-7000-8002-000000000002".parse().unwrap();
    for change in [Change::StartedFailing, Change::WorksAgain] {
        let message = event(change, device, "10.10.0.21".to_owned()).message;
        assert!(!message.contains("10.10.0.21"), "{message}");
        assert!(!message.contains(&device.to_string()), "{message}");
    }
}
