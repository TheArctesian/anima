//! What a beat records from a probe and its evaluation (CHK-003, CHK-008).

use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::{
    fleet::checks::{
        Beat, CheckState,
        probe::{Failure, ProbeResult},
    },
    status::Status::{self, Degraded, Down, Unknown, Up},
};

fn at() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

fn timed(ok: bool) -> ProbeResult {
    let mut probe = if ok {
        ProbeResult::up(Duration::from_millis(42))
    } else {
        ProbeResult::failed(&Failure::TimedOut)
    };
    probe.response_time = Some(Duration::from_millis(42));
    probe
}

fn recorded(probe: ProbeResult, status: Status) -> Option<u32> {
    Beat::of(probe, (CheckState::default(), status), at()).response_time_ms
}

#[test]
fn chk_003_up_and_degraded_keep_the_response_time() {
    assert_eq!(recorded(timed(true), Up), Some(42));
    assert_eq!(recorded(timed(true), Degraded), Some(42));
}

#[test]
fn chk_003_down_and_unknown_drop_the_response_time() {
    assert_eq!(recorded(timed(true), Down), None);
    assert_eq!(recorded(timed(true), Unknown), None);
    assert_eq!(recorded(timed(false), Down), None);
}

#[test]
fn chk_008_an_inverted_up_from_a_failed_probe_has_no_response_time() {
    assert_eq!(recorded(timed(false), Up), None);
}

#[test]
fn chk_003_the_beat_carries_the_probe_details_and_the_state() {
    let mut probe = timed(false);
    probe.resolved_ip = Some("10.10.0.2".parse().unwrap());
    let state = CheckState {
        status: Up,
        failures: 1,
        next_due_at: Some(at()),
    };
    let beat = Beat::of(probe.clone(), (state, Unknown), at());
    assert_eq!(
        (
            beat.at,
            beat.status,
            beat.state,
            beat.resolved_ip,
            beat.message
        ),
        (at(), Unknown, state, probe.resolved_ip, probe.message)
    );
}
