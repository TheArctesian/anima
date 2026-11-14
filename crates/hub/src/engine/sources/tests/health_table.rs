//! SRC-009: every rule of the health table, first match wins, and each boundary of three
//! failures and three intervals.

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};

use crate::engine::sources::{
    health::{HealthFacts, health, paused_by, worsened},
    model::{ErrorKind, Health, PauseReason},
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

const INTERVAL: TimeDelta = TimeDelta::minutes(15);

/// Running, synced a minute ago, no failures: OK.
fn healthy() -> HealthFacts {
    HealthFacts {
        paused: false,
        consumer_on: true,
        last_error_kind: None,
        last_started_at: Some(now() - TimeDelta::minutes(1)),
        last_sync_at: Some(now() - TimeDelta::minutes(1)),
        failure_count: 0,
        interval: INTERVAL,
    }
}

fn assert_table(cases: &[(&str, HealthFacts, Health)]) {
    for (name, facts, want) in cases {
        assert_eq!(health(facts, now()), *want, "{name}");
    }
}

#[test]
fn src_009_paused_then_auth_come_first() {
    assert_table(&[
        ("healthy", healthy(), Health::Ok),
        (
            "paused",
            HealthFacts {
                paused: true,
                ..healthy()
            },
            Health::Paused,
        ),
        (
            "consumer off",
            HealthFacts {
                consumer_on: false,
                ..healthy()
            },
            Health::Paused,
        ),
        (
            "paused beats auth",
            HealthFacts {
                paused: true,
                last_error_kind: Some(ErrorKind::Auth),
                ..healthy()
            },
            Health::Paused,
        ),
        (
            "auth",
            HealthFacts {
                last_error_kind: Some(ErrorKind::Auth),
                failure_count: 5,
                ..healthy()
            },
            Health::NeedsReconnect,
        ),
    ]);
}

#[test]
fn src_009_an_account_never_synced_is_starting_never_synced_or_stalled() {
    let never = HealthFacts {
        last_started_at: None,
        last_sync_at: None,
        ..healthy()
    };
    assert_table(&[
        ("never started", never, Health::NeverSynced),
        (
            "started, never synced",
            HealthFacts {
                last_started_at: Some(now()),
                ..never
            },
            Health::Starting,
        ),
        (
            "never synced, 2 failures",
            HealthFacts {
                last_started_at: Some(now()),
                failure_count: 2,
                ..never
            },
            Health::Starting,
        ),
        (
            "never synced, 3 failures",
            HealthFacts {
                failure_count: 3,
                ..never
            },
            Health::Stalled,
        ),
    ]);
}

#[test]
fn src_009_a_synced_account_is_stalled_from_three_failures() {
    assert_table(&[
        ("healthy", healthy(), Health::Ok),
        (
            "other error kinds are not auth",
            HealthFacts {
                last_error_kind: Some(ErrorKind::RateLimit),
                failure_count: 1,
                ..healthy()
            },
            Health::Ok,
        ),
        (
            "2 failures",
            HealthFacts {
                failure_count: 2,
                ..healthy()
            },
            Health::Ok,
        ),
        (
            "3 failures",
            HealthFacts {
                failure_count: 3,
                ..healthy()
            },
            Health::Stalled,
        ),
    ]);
}

#[test]
fn src_009_three_intervals_without_a_sync_is_the_boundary_of_stalled() {
    let synced = |ago: TimeDelta| HealthFacts {
        last_sync_at: Some(now() - ago),
        ..healthy()
    };
    let three = INTERVAL * 3;
    assert_eq!(
        health(&synced(three), now()),
        Health::Ok,
        "exactly 3 intervals"
    );
    assert_eq!(
        health(&synced(three + TimeDelta::seconds(1)), now()),
        Health::Stalled,
        "a second past"
    );
    assert_eq!(
        health(&synced(three - TimeDelta::seconds(1)), now()),
        Health::Ok
    );
}

#[test]
fn src_009_paused_by_names_the_operator_before_an_extension_that_is_off() {
    let both = HealthFacts {
        paused: true,
        consumer_on: false,
        ..healthy()
    };
    assert_eq!(paused_by(&both), Some(PauseReason::Operator));
    let off = HealthFacts {
        consumer_on: false,
        ..healthy()
    };
    assert_eq!(paused_by(&off), Some(PauseReason::ExtensionOff));
    assert_eq!(paused_by(&healthy()), None);
}

#[test]
fn src_009_only_a_change_into_stalled_or_needs_reconnect_is_a_warning() {
    assert!(worsened(Health::Ok, Health::Stalled));
    assert!(worsened(Health::Stalled, Health::NeedsReconnect));
    assert!(!worsened(Health::Stalled, Health::Stalled));
    assert!(!worsened(Health::Stalled, Health::Ok));
    assert!(!worsened(Health::Starting, Health::NeverSynced));
}
