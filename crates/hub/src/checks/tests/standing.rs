//! The status a check and a device show: worst, groups, pause and maintenance
//! (CHK-004, CHK-006).

use uuid::Uuid;

use crate::{
    checks::status::{Cover, Standing, device_status, maintained, shown},
    ids::{CheckId, DeviceId},
    status::Status::{self, Degraded, Down, Maintenance, Unknown, Up},
};

fn check_id(n: u128) -> CheckId {
    CheckId::from(Uuid::from_u128(n))
}

fn device() -> DeviceId {
    DeviceId::from(Uuid::from_u128(0xd))
}

fn plain(n: u128, status: Status) -> Standing {
    Standing {
        id: check_id(n),
        device: device(),
        parent: None,
        group: false,
        paused: false,
        status,
    }
}

fn child(n: u128, group: u128, status: Status) -> Standing {
    Standing {
        parent: Some(check_id(group)),
        ..plain(n, status)
    }
}

fn group(n: u128) -> Standing {
    Standing {
        group: true,
        ..plain(n, Unknown)
    }
}

fn paused(check: Standing) -> Standing {
    Standing {
        paused: true,
        ..check
    }
}

fn nothing(_: CheckId) -> bool {
    false
}

fn everything(_: CheckId) -> bool {
    true
}

#[test]
fn chk_004_the_device_is_the_worst_of_its_checks() {
    let checks = [plain(1, Up), plain(2, Degraded), plain(3, Up)];
    assert_eq!(device_status(&checks, &nothing), Degraded);
    let checks = [plain(1, Up), plain(2, Down), plain(3, Degraded)];
    assert_eq!(device_status(&checks, &nothing), Down);
}

#[test]
fn chk_004_a_device_with_no_counted_checks_is_unknown() {
    assert_eq!(device_status(&[], &nothing), Unknown);
    assert_eq!(device_status(&[paused(plain(1, Up))], &nothing), Unknown);
    assert_eq!(device_status(&[group(1)], &nothing), Unknown);
}

#[test]
fn chk_005_a_paused_check_is_unknown_and_left_out_of_the_device() {
    let checks = [plain(1, Up), paused(plain(2, Down))];
    assert_eq!(shown(&checks[1], &checks, &nothing), Unknown);
    assert_eq!(device_status(&checks, &nothing), Up);
}

#[test]
fn chk_004_a_group_is_the_worst_of_its_children_that_are_not_paused() {
    let checks = [
        group(1),
        child(2, 1, Up),
        child(3, 1, Degraded),
        paused(child(4, 1, Down)),
        plain(5, Down),
    ];
    assert_eq!(shown(&checks[0], &checks, &nothing), Degraded);
}

#[test]
fn chk_004_a_group_without_counted_children_is_unknown() {
    let checks = [group(1), paused(child(2, 1, Up)), plain(3, Up)];
    assert_eq!(shown(&checks[0], &checks, &nothing), Unknown);
    assert_eq!(device_status(&checks, &nothing), Up);
}

#[test]
fn chk_004_a_paused_group_is_unknown() {
    let checks = [paused(group(1)), child(2, 1, Down)];
    assert_eq!(shown(&checks[0], &checks, &nothing), Unknown);
}

#[test]
fn chk_004_a_group_adds_nothing_beyond_its_children() {
    // An Unknown group with Up children must not make the device Unknown.
    let checks = [group(1), child(2, 1, Up), plain(3, Up)];
    assert_eq!(device_status(&checks, &nothing), Up);
    let checks = [group(1), child(2, 1, Down), plain(3, Up)];
    assert_eq!(device_status(&checks, &nothing), Down);
}

#[test]
fn chk_006_maintenance_turns_only_down_into_maintenance() {
    for status in [Up, Degraded, Maintenance, Unknown] {
        assert_eq!(maintained(status, true), status);
    }
    assert_eq!(maintained(Down, true), Maintenance);
    assert_eq!(maintained(Down, false), Down);
}

#[test]
fn chk_006_a_covered_device_shows_maintenance_instead_of_down() {
    let checks = [plain(1, Up), plain(2, Down)];
    assert_eq!(device_status(&checks, &everything), Maintenance);
    let checks = [plain(1, Degraded), plain(2, Down)];
    assert_eq!(device_status(&checks, &everything), Degraded);
}

#[test]
fn chk_006_coverage_applies_per_check() {
    let checks = [plain(1, Down), plain(2, Down)];
    let only_first = |id: CheckId| id == check_id(1);
    assert_eq!(shown(&checks[0], &checks, &only_first), Maintenance);
    assert_eq!(shown(&checks[1], &checks, &only_first), Down);
    assert_eq!(device_status(&checks, &only_first), Down);
}

#[test]
fn chk_006_a_window_on_a_group_covers_its_children() {
    let checks = [group(1), child(2, 1, Down)];
    let the_group = |id: CheckId| id == check_id(1);
    assert_eq!(shown(&checks[1], &checks, &the_group), Maintenance);
    assert_eq!(shown(&checks[0], &checks, &the_group), Maintenance);
    assert_eq!(device_status(&checks, &the_group), Maintenance);
}

#[test]
fn chk_006_the_manual_flag_covers_every_check_of_the_device() {
    let active = crate::maintenance::ActiveTargets::default();
    let tags = ["infra".to_owned()];
    let cover = |manual| Cover {
        manual,
        active: &active,
        device: device(),
        tags: &tags,
    };
    assert!(cover(true).covers(check_id(1)));
    assert!(!cover(false).covers(check_id(1)));
}
