//! What the active windows cover (MW-003).

use super::*;
use crate::{
    fleet::maintenance::{ActiveTargets, Target, model::Window},
    ids::{CheckId, DeviceId, WindowId},
};

const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";
const OTHER_DEVICE: &str = "0190a000-0000-7000-8002-000000000002";
const CHECK: &str = "0190a000-0000-7000-8003-000000000001";
const OTHER_CHECK: &str = "0190a000-0000-7000-8003-000000000002";

fn device(text: &str) -> DeviceId {
    text.parse().unwrap()
}

fn check(text: &str) -> CheckId {
    text.parse().unwrap()
}

fn tags(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// A manual window that is on or off, applying to `targets`.
fn manual(active: bool, targets: Vec<Target>) -> Window {
    Window {
        id: WindowId::from(uuid::Uuid::nil()),
        name: "w".into(),
        timezone: UTC,
        rule: WindowRule::Manual,
        active_manual: active,
        targets,
        created_at: at("2026-11-04T09:00:00Z"),
        updated_at: at("2026-11-04T09:00:00Z"),
    }
}

fn covering(targets: Vec<Target>) -> ActiveTargets {
    ActiveTargets::of(&[manual(true, targets)], at("2026-11-04T09:00:00Z"))
}

#[test]
fn mw_003_a_device_target_covers_that_device_only() {
    let active = covering(vec![Target::Device(device(DEVICE))]);
    assert!(active.covers(device(DEVICE), None, &[]));
    assert!(active.covers(device(DEVICE), Some(check(CHECK)), &[]));
    assert!(!active.covers(device(OTHER_DEVICE), None, &[]));
}

#[test]
fn mw_003_a_check_target_covers_that_check_and_not_its_device() {
    let active = covering(vec![Target::Check(check(CHECK))]);
    assert!(active.covers(device(DEVICE), Some(check(CHECK)), &[]));
    assert!(!active.covers(device(DEVICE), Some(check(OTHER_CHECK)), &[]));
    assert!(!active.covers(device(DEVICE), None, &[]));
}

#[test]
fn mw_003_a_tag_target_covers_a_device_with_that_tag() {
    let active = covering(vec![Target::Tag("infra".into())]);
    assert!(active.covers(device(DEVICE), None, &tags(&["lab", "infra"])));
    assert!(active.covers(device(DEVICE), Some(check(CHECK)), &tags(&["infra"])));
    assert!(!active.covers(device(DEVICE), None, &tags(&["lab"])));
    assert!(!active.covers(device(DEVICE), None, &[]));
}

#[test]
fn mw_003_a_window_that_is_not_active_covers_nothing() {
    let off = manual(false, vec![Target::Device(device(DEVICE))]);
    let active = ActiveTargets::of(&[off], at("2026-11-04T09:00:00Z"));
    assert!(!active.covers(device(DEVICE), None, &[]));
    assert_eq!(active, ActiveTargets::default());
}

#[test]
fn mw_003_a_window_may_apply_to_several_targets_of_different_types() {
    let active = covering(vec![
        Target::Device(device(DEVICE)),
        Target::Check(check(CHECK)),
        Target::Tag("infra".into()),
    ]);
    assert!(active.covers(device(DEVICE), None, &[]));
    assert!(active.covers(device(OTHER_DEVICE), Some(check(CHECK)), &[]));
    assert!(active.covers(device(OTHER_DEVICE), None, &tags(&["infra"])));
    assert!(!active.covers(device(OTHER_DEVICE), None, &[]));
}
