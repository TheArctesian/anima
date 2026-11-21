//! The device policy rules (CMP-002, CAP-010 to CAP-013) with a set time.

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use daemon_core::hardware::PowerSource;

use super::{
    DRAIN_LIMIT, DevicePolicy, Drain, NodeState, PolicyError, Stop, WorkSwitch, quiet_hours,
    takes_model_work,
};

fn at(hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 20, hour, minute, 0).unwrap()
}

fn idle_on_mains() -> NodeState {
    NodeState {
        power: Some(PowerSource::Mains),
        operator_active: Some(false),
    }
}

#[test]
fn cmp_002_a_machine_whose_switch_is_off_gets_no_model_work() {
    let policy = DevicePolicy::default();
    let verdict = |switch| takes_model_work(&policy, switch, idle_on_mains(), at(12, 0));

    assert_eq!(verdict(WorkSwitch::OffAtMachine), Err(Stop::SwitchOff));
    assert_eq!(verdict(WorkSwitch::OffInHub), Err(Stop::OffInHub));
    assert_eq!(verdict(WorkSwitch::Unknown), Err(Stop::NoSwitch));
    assert_eq!(verdict(WorkSwitch::On), Ok(()));
}

#[test]
fn cap_010_quiet_hours_stop_new_work_also_past_midnight() {
    let policy = DevicePolicy {
        quiet_hours: Some(quiet_hours("22:00", "07:00", "UTC").unwrap()),
        ..DevicePolicy::default()
    };
    let verdict = |now| takes_model_work(&policy, WorkSwitch::On, idle_on_mains(), now);

    assert_eq!(verdict(at(22, 0)), Err(Stop::QuietHours));
    assert_eq!(verdict(at(3, 30)), Err(Stop::QuietHours));
    assert_eq!(verdict(at(7, 0)), Ok(()));
    assert_eq!(verdict(at(21, 59)), Ok(()));
}

#[test]
fn cap_010_quiet_hours_are_read_in_their_time_zone() {
    let policy = DevicePolicy {
        quiet_hours: Some(quiet_hours("09:00", "17:00", "America/New_York").unwrap()),
        ..DevicePolicy::default()
    };
    let verdict = |now| takes_model_work(&policy, WorkSwitch::On, idle_on_mains(), now);

    assert_eq!(
        verdict(at(15, 0)),
        Err(Stop::QuietHours),
        "10:00 in New York"
    );
    assert_eq!(verdict(at(9, 0)), Ok(()), "04:00 in New York");
}

#[test]
fn cap_011_a_portable_computer_on_battery_takes_no_model_work_by_default() {
    let on_battery = NodeState {
        power: Some(PowerSource::Battery),
        operator_active: Some(false),
    };
    let allowed = DevicePolicy {
        mains_only: false,
        ..DevicePolicy::default()
    };

    let default = takes_model_work(
        &DevicePolicy::default(),
        WorkSwitch::On,
        on_battery,
        at(12, 0),
    );
    let changed = takes_model_work(&allowed, WorkSwitch::On, on_battery, at(12, 0));

    assert_eq!(default, Err(Stop::OnBattery));
    assert_eq!(changed, Ok(()));
}

#[test]
fn cap_012_an_active_operator_stops_new_work_only_when_the_device_says_so() {
    let active = NodeState {
        power: Some(PowerSource::Mains),
        operator_active: Some(true),
    };
    let idle_only = DevicePolicy {
        idle_only: true,
        ..DevicePolicy::default()
    };

    let set = takes_model_work(&idle_only, WorkSwitch::On, active, at(12, 0));
    let default = takes_model_work(&DevicePolicy::default(), WorkSwitch::On, active, at(12, 0));
    let unknown = takes_model_work(&idle_only, WorkSwitch::On, NodeState::default(), at(12, 0));

    assert_eq!(set, Err(Stop::OperatorActive));
    assert_eq!(default, Ok(()));
    assert_eq!(unknown, Ok(()), "an older daemon reports no activity");
}

#[test]
fn cap_013_a_policy_that_stops_a_node_drains_it_in_60_seconds_or_fewer() {
    let start = at(12, 0);
    let mut clock = start;
    let mut node = Drain::Serving.next(true, clock);

    node = node.next(false, clock);
    let drained_at = loop {
        assert!(!node.takes_new(), "no new request while it drains");
        if !node.runs_current(clock) {
            break clock;
        }
        clock += TimeDelta::seconds(1);
        node = node.next(false, clock);
    };

    assert!(drained_at - start <= DRAIN_LIMIT);
    assert_eq!(DRAIN_LIMIT, TimeDelta::seconds(60));
    assert_eq!(node, Drain::Stopped);
}

#[test]
fn cap_013_a_node_permitted_again_serves_at_once() {
    let draining = Drain::Serving.next(false, at(12, 0));

    let back = draining.next(true, at(12, 0) + TimeDelta::seconds(10));

    assert!(draining.runs_current(at(12, 0) + TimeDelta::seconds(59)));
    assert!(back.takes_new());
}

#[test]
fn cap_010_a_policy_out_of_range_is_refused() {
    let with = |memory_share, disk_share| {
        DevicePolicy {
            memory_share,
            disk_share,
            ..DevicePolicy::default()
        }
        .checked()
    };

    assert_eq!(with(0, 50), Err(PolicyError::MemoryShare));
    assert_eq!(with(50, 101), Err(PolicyError::DiskShare));
    assert_eq!(quiet_hours("25:00", "07:00", "UTC"), Err(PolicyError::Time));
    assert_eq!(
        quiet_hours("22:00", "07:00", "Mars/Base"),
        Err(PolicyError::Zone)
    );
    let empty = DevicePolicy {
        quiet_hours: Some(quiet_hours("07:00", "07:00", "UTC").unwrap()),
        ..DevicePolicy::default()
    };
    assert_eq!(empty.checked(), Err(PolicyError::EmptyQuietHours));
}
