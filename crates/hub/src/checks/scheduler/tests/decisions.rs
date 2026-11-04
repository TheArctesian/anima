//! The pure decisions: first-run spread, lateness, missed pushes and event wording (CHK-001,
//! CHK-002, CHK-010).

use std::collections::HashSet;

use chrono::{DateTime, TimeDelta, Utc};

use super::support::{DOCKER_HOST, device, rng, settings};
use crate::{
    checks::{
        CheckState, Due,
        config::CheckConfig,
        evaluate,
        scheduler::schedule::{
            Watchdog, first_due, is_late, missed_push, severity_of, status_message,
        },
    },
    events::Severity,
    ids::CheckId,
    status::Status,
};

fn now() -> DateTime<Utc> {
    "2026-11-04T12:00:00Z".parse().unwrap()
}

fn due(config: CheckConfig, next_due_at: Option<DateTime<Utc>>) -> Due {
    Due {
        id: CheckId::from(uuid::Uuid::now_v7()),
        device: device(DOCKER_HOST),
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

#[test]
fn chk_001_first_runs_are_spread_over_the_interval() {
    let mut rng = rng();
    let check = due(tcp(), None);
    let runs: Vec<_> = (0..50)
        .map(|_| first_due(&check, now(), &mut rng))
        .collect();
    let interval = TimeDelta::seconds(10);
    assert!(
        runs.iter().all(|at| *at >= now() && *at < now() + interval),
        "{runs:?}"
    );
    assert!(runs.iter().collect::<HashSet<_>>().len() > 1, "{runs:?}");
}

#[test]
fn chk_001_a_push_check_first_waits_its_whole_interval() {
    let mut rng = rng();
    let check = due(CheckConfig::Push, None);
    assert_eq!(
        first_due(&check, now(), &mut rng),
        now() + TimeDelta::seconds(10)
    );
}

#[test]
fn chk_010_a_check_is_late_only_beyond_2_intervals() {
    let at = |seconds| due(tcp(), Some(now() - TimeDelta::seconds(seconds)));
    assert!(!is_late(&at(20), now()));
    assert!(is_late(&at(21), now()));
    assert!(!is_late(&due(tcp(), None), now()));
}

#[test]
fn chk_010_the_watchdog_reports_once_per_episode_and_again_after_a_run() {
    let mut watchdog = Watchdog::default();
    let late = due(tcp(), Some(now() - TimeDelta::seconds(30)));
    assert!(watchdog.should_report(&late, now()));
    assert!(!watchdog.should_report(&late, now() + TimeDelta::seconds(1)));
    let on_time = due(tcp(), Some(now()));
    assert!(!watchdog.should_report(&on_time, now()));
    watchdog.clear(late.id);
    assert!(watchdog.should_report(&late, now() + TimeDelta::seconds(2)));
}

#[test]
fn chk_002_a_missed_push_is_a_failed_attempt_that_waits_for_the_retries() {
    let check = due(CheckConfig::Push, Some(now()));
    let (first, status) = evaluate(&check.state, missed_push(), &check.settings, now());
    assert_eq!(
        (first.status, first.failures, status),
        (Status::Unknown, 1, Status::Unknown)
    );
    let (second, _) = evaluate(&first, missed_push(), &check.settings, now());
    let (third, status) = evaluate(&second, missed_push(), &check.settings, now());
    assert_eq!((third.status, status), (Status::Down, Status::Down));
    assert_eq!(missed_push().message.as_deref(), Some("no push received"));
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
