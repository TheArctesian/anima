//! The status rule of a domain expiry check (D-29): Up at or above the warning days, Degraded
//! below them, Down after expiry, Unknown without an expiry date.

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};

use crate::{
    checks::{
        evaluation::{CheckSettings, CheckState, evaluate},
        probe::{
            outcome::{Failure, Judgement},
            rdap::expiry::judge,
        },
    },
    status::Status,
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 8, 12, 0, 0).unwrap()
}

/// The status a check with no retries records for the expiry `offset` from now.
fn status(offset: Option<TimeDelta>, warn_days: u32) -> Status {
    let settings = CheckSettings {
        retries: 0,
        ..CheckSettings::default()
    };
    let result = judge(offset.map(|offset| now() + offset), warn_days, now());
    evaluate(&CheckState::default(), &result, &settings, now()).1
}

#[test]
fn d_29_the_status_follows_the_days_left() {
    let day = TimeDelta::days(1);
    let cases = [
        (Some(day * 400), 30, Status::Up),
        (Some(day * 30), 30, Status::Up),
        (Some(day * 30 - TimeDelta::seconds(1)), 30, Status::Degraded),
        (Some(day * 12), 30, Status::Degraded),
        (Some(TimeDelta::hours(1)), 30, Status::Degraded),
        (Some(TimeDelta::seconds(1)), 1, Status::Degraded),
        (Some(day), 1, Status::Up),
        (Some(TimeDelta::zero()), 30, Status::Down),
        (Some(-TimeDelta::seconds(1)), 30, Status::Down),
        (Some(-day * 90), 30, Status::Down),
        (None, 30, Status::Unknown),
    ];
    for (offset, warn_days, expected) in cases {
        assert_eq!(
            status(offset, warn_days),
            expected,
            "{offset:?} {warn_days}"
        );
    }
}

#[test]
fn d_29_the_result_carries_the_date_and_no_response_time() {
    let not_after = now() + TimeDelta::days(12);

    let expiring = judge(Some(not_after), 30, now());
    let fine = judge(Some(not_after), 10, now());
    let expired = judge(Some(now()), 30, now());
    let unknown = judge(None, 30, now());

    assert_eq!(expiring.domain_not_after, Some(not_after));
    assert_eq!(expiring.judgement, Judgement::Expiring);
    assert!(expiring.ok);
    assert_eq!(
        expiring.message.as_deref(),
        Some("the domain registration expires within its warning days")
    );
    assert_eq!(fine.domain_not_after, Some(not_after));
    assert_eq!((fine.ok, fine.message.as_deref()), (true, None));
    assert_eq!(expired.domain_not_after, Some(now()));
    assert!(!expired.ok);
    assert_eq!(expired.message, Some(Failure::DomainExpired.to_string()));
    assert_eq!(unknown.domain_not_after, None);
    assert_eq!(unknown.judgement, Judgement::Unjudged);
    for result in [expiring, fine, expired, unknown] {
        assert_eq!(result.response_time, None);
        assert_eq!(result.resolved_ip, None);
    }
}

#[test]
fn d_29_an_expired_domain_is_down_only_after_the_retries() {
    let settings = CheckSettings::default();
    let expired = judge(Some(now() - TimeDelta::days(1)), 30, now());

    let mut state = CheckState::default();
    let mut recorded = Vec::new();
    for _ in 0..=settings.retries {
        let (next, status) = evaluate(&state, &expired, &settings, now());
        state = next;
        recorded.push(status);
    }

    assert_eq!(
        recorded,
        [
            Status::Unknown,
            Status::Unknown,
            Status::Unknown,
            Status::Down
        ]
    );
}
