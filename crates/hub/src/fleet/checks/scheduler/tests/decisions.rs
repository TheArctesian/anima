//! The pure decisions: first-run spread, the domain expiry floor, lateness, missed pushes
//! and event wording (CHK-001, CHK-002, CHK-010, DB-005, R-102).

use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};

use super::support::{DOCKER_HOST, device, rng, settings};
use crate::{
    fleet::{
        checks::{
            CheckSettings, CheckState, Due,
            config::CheckConfig,
            evaluate,
            probe::{Failure, ProbeResult},
            scheduler::schedule::{
                Timing, Watchdog, is_late, missed_push, needs_new_due, new_due, not_before_floor,
                severity_of, status_message,
            },
        },
        events::Severity,
    },
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
        last_beat_at: None,
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
    let runs: Vec<_> = (0..50).map(|_| new_due(&check, now(), &mut rng)).collect();
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
    assert_eq!(new_due(&check, now(), &mut rng), now() + seconds(10));
}

#[test]
fn chk_001_checks_due_before_the_start_are_spread_and_later_ones_run() {
    let started = now() - seconds(60);
    assert!(needs_new_due(&due(tcp(), None), started));
    assert!(needs_new_due(
        &due(tcp(), Some(started - seconds(1))),
        started
    ));
    assert!(!needs_new_due(&due(tcp(), Some(started)), started));
}

fn domain() -> CheckConfig {
    CheckConfig::DomainExpiry {
        domain: "example.net".to_owned(),
        warn_days: 30,
        timeout_ms: 10_000,
    }
}

/// A check of `config` due at `next_due_at`, with an hourly interval and its last beat
/// `ago` seconds before `now()`.
fn hourly(config: CheckConfig, next_due_at: Option<DateTime<Utc>>, ago: Option<i64>) -> Due {
    let mut check = due(config, next_due_at);
    check.settings.interval_seconds = 3600;
    check.last_beat_at = ago.map(|ago| now() - seconds(ago));
    check
}

#[test]
fn r_102_a_domain_check_is_never_due_before_its_last_beat_plus_the_interval() {
    let floor = now() + seconds(3000);
    let mut rng = rng();
    for next_due_at in [None, Some(now() - seconds(1)), Some(now())] {
        let check = hourly(domain(), next_due_at, Some(600));
        assert!(
            needs_new_due(&check, now() - seconds(60)),
            "{next_due_at:?}"
        );
        let runs: Vec<_> = (0..50).map(|_| new_due(&check, now(), &mut rng)).collect();
        assert!(runs.iter().all(|at| *at >= floor), "{runs:?}");
    }
    let at_floor = hourly(domain(), Some(floor), Some(600));
    assert!(!needs_new_due(&at_floor, now() - seconds(60)));
    let retry = now() + seconds(5);
    assert_eq!(
        not_before_floor(&at_floor, Some(now()), retry),
        now() + seconds(3600)
    );
    assert_eq!(
        not_before_floor(&at_floor, at_floor.last_beat_at, retry),
        floor
    );
}

#[test]
fn r_102_a_domain_check_with_no_beat_may_run_at_once() {
    let started = now() - seconds(60);
    assert!(!needs_new_due(
        &hourly(domain(), Some(now()), None),
        started
    ));
    let mut rng = rng();
    let unscheduled = hourly(domain(), None, None);
    let runs: Vec<_> = (0..50)
        .map(|_| new_due(&unscheduled, now(), &mut rng))
        .collect();
    assert!(runs.iter().any(|at| *at < now() + seconds(600)), "{runs:?}");
    assert_eq!(not_before_floor(&unscheduled, None, now()), now());
}

#[test]
fn r_102_other_check_types_have_no_floor() {
    let started = now() - seconds(60);
    let port = hourly(tcp(), Some(now()), Some(600));
    assert!(!needs_new_due(&port, started));
    let retry = now() + seconds(5);
    assert_eq!(not_before_floor(&port, Some(now()), retry), retry);
    let mut rng = rng();
    let unscheduled = hourly(tcp(), None, Some(600));
    let runs: Vec<_> = (0..50)
        .map(|_| new_due(&unscheduled, now(), &mut rng))
        .collect();
    assert!(
        runs.iter().any(|at| *at < now() + seconds(3000)),
        "{runs:?}"
    );
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

/// CHK-002 with the default settings (spec 6.4): a refused
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
