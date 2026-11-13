//! Retries, invert and the Degraded limit (CHK-002, CHK-008).

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

use crate::{
    fleet::checks::{
        CheckSettings, CheckState, evaluate,
        probe::{Failure, ProbeResult},
    },
    status::Status::{self, Degraded, Down, Unknown, Up},
};

fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

fn settings(retries: u8) -> CheckSettings {
    CheckSettings {
        retries,
        ..CheckSettings::default()
    }
}

fn up(ms: u64) -> ProbeResult {
    ProbeResult::up(Duration::from_millis(ms))
}

fn failed() -> ProbeResult {
    ProbeResult::failed(&Failure::TimedOut)
}

/// Runs the probes in order from a fresh state; returns each heartbeat status and the last state.
fn run(cfg: &CheckSettings, probes: Vec<ProbeResult>) -> (Vec<Status>, CheckState) {
    let mut state = CheckState::default();
    let mut beats = Vec::new();
    for probe in probes {
        let (next, beat) = evaluate(&state, &probe, cfg, now());
        beats.push(beat);
        state = next;
    }
    (beats, state)
}

#[test]
fn chk_002_with_no_retries_the_first_failure_is_down() {
    let (beats, state) = run(&settings(0), vec![failed()]);
    assert_eq!(beats, [Down]);
    assert_eq!((state.status, state.failures), (Down, 1));
}

#[test]
fn chk_002_with_one_retry_down_comes_on_the_second_failure() {
    let (beats, state) = run(&settings(1), vec![up(5), failed(), failed()]);
    assert_eq!(beats, [Up, Unknown, Down]);
    assert_eq!(state.status, Down);
}

#[test]
fn chk_002_with_three_retries_down_comes_after_four_failures() {
    let (beats, _) = run(&settings(3), vec![failed(); 4]);
    assert_eq!(beats, [Unknown, Unknown, Unknown, Down]);
}

#[test]
fn chk_002_a_held_failure_keeps_the_previous_status_and_waits_the_retry_interval() {
    let cfg = settings(3);
    let up_state = evaluate(&CheckState::default(), &up(5), &cfg, now()).0;
    let (state, beat) = evaluate(&up_state, &failed(), &cfg, now());
    assert_eq!(beat, Unknown);
    assert_eq!(state.status, Up);
    assert_eq!(state.failures, 1);
    assert_eq!(state.next_due_at, Some(now() + TimeDelta::seconds(20)));
}

#[test]
fn chk_002_down_and_success_wait_the_interval() {
    let (_, down) = run(&settings(0), vec![failed()]);
    assert_eq!(down.next_due_at, Some(now() + TimeDelta::seconds(60)));
    let (_, up_state) = run(&settings(0), vec![up(5)]);
    assert_eq!(up_state.next_due_at, Some(now() + TimeDelta::seconds(60)));
}

#[test]
fn chk_002_recovery_resets_the_failures() {
    let (beats, state) = run(&settings(1), vec![failed(), failed(), up(5), failed()]);
    assert_eq!(beats, [Unknown, Down, Up, Unknown]);
    assert_eq!((state.status, state.failures), (Up, 1));
}

#[test]
fn chk_002_a_held_failure_after_degraded_stays_degraded_until_down() {
    let cfg = CheckSettings {
        degraded_threshold_ms: 10,
        ..settings(1)
    };
    let mut state = CheckState::default();
    let mut shown = Vec::new();
    for probe in [up(50), failed(), failed()] {
        let (next, _) = evaluate(&state, &probe, &cfg, now());
        shown.push(next.status);
        state = next;
    }
    assert_eq!(shown, [Degraded, Degraded, Down]);
}

#[test]
fn chk_002_ten_retries_hold_ten_failures_and_the_eleventh_is_down() {
    let (beats, state) = run(&settings(10), vec![failed(); 11]);
    assert_eq!(beats[..10], [Unknown; 10]);
    assert_eq!(beats[10], Down);
    assert_eq!(state.failures, 11);
}

#[test]
fn chk_008_invert_makes_a_failed_probe_up() {
    let cfg = CheckSettings {
        invert: true,
        ..settings(0)
    };
    let (beats, state) = run(&cfg, vec![failed()]);
    assert_eq!(beats, [Up]);
    assert_eq!(state.failures, 0);
}

#[test]
fn chk_008_invert_makes_a_successful_probe_a_failure() {
    let cfg = CheckSettings {
        invert: true,
        ..settings(0)
    };
    assert_eq!(run(&cfg, vec![up(5)]).0, [Down]);
}

#[test]
fn chk_008_an_inverted_success_is_never_degraded() {
    let cfg = CheckSettings {
        invert: true,
        degraded_threshold_ms: 1,
        ..settings(0)
    };
    let mut slow_failure = failed();
    slow_failure.response_time = Some(Duration::from_secs(5));
    assert_eq!(run(&cfg, vec![slow_failure]).0, [Up]);
}

#[test]
fn a_success_slower_than_the_threshold_is_degraded() {
    let cfg = CheckSettings {
        degraded_threshold_ms: 100,
        ..settings(0)
    };
    assert_eq!(run(&cfg, vec![up(100)]).0, [Up]);
    assert_eq!(run(&cfg, vec![up(101)]).0, [Degraded]);
    let (_, state) = run(&cfg, vec![failed(), up(101)]);
    assert_eq!((state.status, state.failures), (Degraded, 0));
}

fn starting() -> ProbeResult {
    ProbeResult::starting(&Failure::ContainerStarting)
}

fn unjudged() -> ProbeResult {
    ProbeResult::unjudged(&Failure::NeedsLink)
}

fn inverted(retries: u8) -> CheckSettings {
    CheckSettings {
        invert: true,
        ..settings(retries)
    }
}

#[test]
fn d_2_a_starting_result_is_degraded_and_resets_the_failures() {
    let cfg = settings(3);
    let prev = CheckState {
        status: Down,
        failures: 4,
        next_due_at: None,
    };
    let (state, beat) = evaluate(&prev, &starting(), &cfg, now());
    assert_eq!(beat, Degraded);
    assert_eq!(
        state,
        CheckState {
            status: Degraded,
            failures: 0,
            next_due_at: Some(now() + TimeDelta::seconds(60)),
        }
    );
}

#[test]
fn d_2_an_inverted_starting_result_is_a_failure_because_the_container_runs() {
    let (beats, state) = run(&inverted(0), vec![starting()]);
    assert_eq!(beats, [Down]);
    assert_eq!((state.status, state.failures), (Down, 1));
    let (beats, state) = run(&inverted(1), vec![starting(), starting()]);
    assert_eq!(beats, [Unknown, Down]);
    assert_eq!(state.failures, 2);
}

#[test]
fn d_2_an_unjudged_result_is_unknown_with_failures_reset_and_the_interval_next() {
    let cfg = settings(0);
    let prev = CheckState {
        status: Down,
        failures: 3,
        next_due_at: None,
    };
    let (state, beat) = evaluate(&prev, &unjudged(), &cfg, now());
    assert_eq!(beat, Unknown);
    assert_eq!(
        state,
        CheckState {
            status: Unknown,
            failures: 0,
            next_due_at: Some(now() + TimeDelta::seconds(60)),
        }
    );
}

#[test]
fn d_2_invert_does_not_apply_to_an_unjudged_result() {
    let cfg = inverted(0);
    let (state, beat) = evaluate(&CheckState::default(), &unjudged(), &cfg, now());
    assert_eq!(beat, Unknown);
    assert_eq!((state.status, state.failures), (Unknown, 0));
}

#[test]
fn d_2_an_unjudged_result_is_not_a_retry_and_keeps_its_message() {
    let probe = unjudged();
    assert!(!probe.ok);
    assert_eq!(
        probe.message.as_deref(),
        Some("this check runs through the SSH link")
    );
    let cfg = settings(2);
    let (beats, state) = run(&cfg, vec![failed(), unjudged()]);
    assert_eq!(beats, [Unknown, Unknown]);
    assert_eq!(state.failures, 0);
    assert_eq!(state.next_due_at, Some(now() + TimeDelta::seconds(60)));
}
