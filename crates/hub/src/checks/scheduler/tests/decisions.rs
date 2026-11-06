//! The pure decisions: first-run spread, lateness, the daily gate, missed pushes and event
//! wording (CHK-001, CHK-002, CHK-010, DB-005).

use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};

use super::support::{DOCKER_HOST, device, rng, settings};
use crate::{
    checks::{
        CheckSettings, CheckState, Due,
        config::CheckConfig,
        evaluate,
        probe::{Failure, ProbeResult},
        scheduler::schedule::{
            Timing, Watchdog, daily_due, first_due, is_late, missed_push, needs_spreading,
            next_daily, severity_of, status_message,
        },
    },
    events::Severity,
    ids::CheckId,
    status::Status,
};

fn now() -> DateTime<Utc> {
    "2026-11-04T12:00:00Z".parse().unwrap()
}

fn seconds(n: i64) -> TimeDelta {
    TimeDelta::seconds(n)
}

fn due(config: CheckConfig, next_due_at: Option<DateTime<Utc>>) -> Due {
    Due {
        id: CheckId::from(uuid::Uuid::now_v7()),
        device: device(DOCKER_HOST),
        parent: None,
        config: Some(config),
        settings: settings(2),
        state: CheckState {
            next_due_at,
            ..CheckState::default()
        },
    }
}

fn tcp() -> CheckConfig {
    CheckConfig::Tcp {
        port: 22,
        timeout_ms: 1000,
    }
}

/// Whether a check due `ago` seconds before `now()` is late, with the scheduler started
/// `started` seconds before.
fn late(ago: i64, started: i64) -> bool {
    let check = due(tcp(), Some(now() - seconds(ago)));
    is_late(Timing::from(&check), now() - seconds(started), now())
}

#[test]
fn chk_001_first_runs_are_spread_over_the_interval() {
    let mut rng = rng();
    let check = due(tcp(), None);
    let runs: Vec<_> = (0..50)
        .map(|_| first_due(&check, now(), &mut rng))
        .collect();
    assert!(
        runs.iter()
            .all(|at| *at >= now() && *at < now() + seconds(10)),
        "{runs:?}"
    );
    assert!(runs.iter().collect::<HashSet<_>>().len() > 1, "{runs:?}");
}

#[test]
fn chk_001_a_push_check_first_waits_its_whole_interval() {
    let mut rng = rng();
    let check = due(CheckConfig::Push, None);
    assert_eq!(first_due(&check, now(), &mut rng), now() + seconds(10));
}

#[test]
fn chk_001_checks_due_before_the_start_are_spread_and_later_ones_run() {
    let started = now() - seconds(60);
    assert!(needs_spreading(&due(tcp(), None), started));
    assert!(needs_spreading(
        &due(tcp(), Some(started - seconds(1))),
        started
    ));
    assert!(!needs_spreading(&due(tcp(), Some(started)), started));
}

#[test]
fn chk_010_a_check_is_late_only_beyond_2_intervals() {
    assert!(!late(20, 600));
    assert!(late(21, 600));
    let unscheduled = due(tcp(), None);
    assert!(!is_late(
        Timing::from(&unscheduled),
        now() - seconds(600),
        now()
    ));
}

#[test]
fn chk_010_lateness_counts_from_the_start_after_a_hub_restart() {
    assert!(!late(3600, 20));
    assert!(late(3600, 21));
}

#[test]
fn chk_010_the_watchdog_reports_once_per_episode_and_again_after_a_clear() {
    let mut watchdog = Watchdog::default();
    let check = CheckId::from(uuid::Uuid::now_v7());
    assert!(watchdog.should_report(check, true));
    assert!(!watchdog.should_report(check, true));
    assert!(!watchdog.should_report(CheckId::from(uuid::Uuid::now_v7()), false));
    watchdog.clear(check);
    assert!(watchdog.should_report(check, true));
    watchdog.retain(&HashSet::new());
    assert!(watchdog.should_report(check, true));
}

#[test]
fn db_005_the_daily_jobs_run_first_then_daily_or_hourly_after_a_failure() {
    assert!(daily_due(None, now()));
    let after_success = next_daily(true, now());
    assert_eq!(after_success, now() + TimeDelta::hours(24));
    assert!(!daily_due(
        Some(after_success),
        now() + TimeDelta::hours(23)
    ));
    assert!(daily_due(Some(after_success), now() + TimeDelta::hours(24)));
    assert_eq!(next_daily(false, now()), now() + TimeDelta::hours(1));
}

#[test]
fn cq_021_the_daily_jobs_run_again_when_the_clock_goes_back_more_than_a_day() {
    let next = next_daily(true, now());
    assert!(!daily_due(Some(next), now()));
    assert!(daily_due(Some(next), now() - seconds(1)));
}

#[test]
fn chk_002_a_missed_push_is_a_failed_attempt_that_waits_for_the_retries() {
    let check = due(CheckConfig::Push, Some(now()));
    let missed = missed_push();
    let (first, status) = evaluate(&check.state, &missed, &check.settings, now());
    assert_eq!(
        (first.status, first.failures, status),
        (Status::Unknown, 1, Status::Unknown)
    );
    let (second, _) = evaluate(&first, &missed, &check.settings, now());
    let (third, status) = evaluate(&second, &missed, &check.settings, now());
    assert_eq!((third.status, status), (Status::Down, Status::Down));
    assert_eq!(missed.message.as_deref(), Some("no push received"));
}

/// How long after its last Up beat a device that stopped right after it is known to be
/// Down, by `evaluate` alone: each attempt starts when the previous one asked, and the
/// Down result is known when the last attempt's probe ends, `probe_time` after its start.
fn known_down_after(settings: &CheckSettings, probe_time: TimeDelta) -> TimeDelta {
    let up = ProbeResult::up(Duration::from_millis(5));
    let (mut state, _) = evaluate(&CheckState::default(), &up, settings, now());
    let silent = ProbeResult::failed(&Failure::TimedOut);
    for _ in 0..16 {
        let at = state.next_due_at.expect("a due time");
        state = evaluate(&state, &silent, settings, at).0;
        if state.status == Status::Down {
            return at + probe_time - now();
        }
    }
    panic!("never Down: {state:?}");
}

/// The default timeout of a check type, from its JSON form with only the required keys.
fn default_timeout(json: serde_json::Value) -> TimeDelta {
    let config: CheckConfig = serde_json::from_value(json).unwrap();
    TimeDelta::from_std(config.timeout()).unwrap()
}

/// Acceptance criterion 7 of WP-17 with the default settings (spec 6.4): a refused
/// connect fails at once, so the device is Down exactly 2 intervals after its last Up
/// beat; a device that drops packets adds one probe timeout to the last attempt.
#[test]
fn chk_002_with_the_defaults_a_stopped_device_is_down_within_2_intervals_plus_a_timeout() {
    let defaults = CheckSettings::default();
    let two_intervals = seconds(2 * i64::from(defaults.interval_seconds));
    assert_eq!(two_intervals, seconds(120));

    assert_eq!(
        known_down_after(&defaults, TimeDelta::zero()),
        two_intervals
    );

    let tcp = default_timeout(serde_json::json!({ "type": "tcp", "port": 22 }));
    assert_eq!(tcp, seconds(5));
    assert_eq!(known_down_after(&defaults, tcp), seconds(125));

    let icmp = default_timeout(serde_json::json!({ "type": "icmp" }));
    assert_eq!(icmp, seconds(2));
    assert_eq!(known_down_after(&defaults, icmp), seconds(122));
}

#[test]
fn log_004_status_change_events_are_error_for_down_and_warning_for_degraded() {
    let severities = [
        Status::Down,
        Status::Degraded,
        Status::Up,
        Status::Maintenance,
    ]
    .map(severity_of);
    let expected = [
        Severity::Error,
        Severity::Warning,
        Severity::Info,
        Severity::Info,
    ];
    assert_eq!(severities, expected);
    assert_eq!(status_message("nas", Status::Down), "nas is Down");
}
