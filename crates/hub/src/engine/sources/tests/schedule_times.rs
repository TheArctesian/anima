//! SRC-021: the poll interval's floor, the backoff after failures, the rate-limit wait, which
//! due accounts run, and the sync-now rules.

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use uuid::Uuid;

use crate::{
    engine::sources::{
        Error,
        acquisition::verdict::{Limits, Verdict, judge},
        controls::sync_now,
        model::Account,
        schedule::{Due, MAX_BACKOFF, interval, next, runnable},
    },
    ids::SourceAccountId,
};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap()
}

#[test]
fn src_021_the_interval_is_never_under_fifteen_minutes() {
    assert_eq!(interval(60), TimeDelta::minutes(15));
    assert_eq!(interval(900), TimeDelta::minutes(15));
    assert_eq!(interval(3600), TimeDelta::hours(1));
}

#[test]
fn src_021_success_waits_one_interval_and_failures_double_up_to_six_hours() {
    let every = TimeDelta::minutes(15);
    let waits: Vec<TimeDelta> = (0..=7)
        .map(|failures| next(now(), every, failures, None) - now())
        .collect();
    let minutes = |m| TimeDelta::minutes(m);
    assert_eq!(
        waits,
        [
            minutes(15),
            minutes(30),
            minutes(60),
            minutes(120),
            minutes(240),
            MAX_BACKOFF,
            MAX_BACKOFF,
            MAX_BACKOFF
        ]
    );
    assert_eq!(
        next(now(), every, u32::MAX, None) - now(),
        MAX_BACKOFF,
        "no overflow"
    );
}

#[test]
fn src_021_a_rate_limit_waits_for_the_later_of_its_end_and_the_backoff() {
    let every = TimeDelta::minutes(15);
    let far = now() + TimeDelta::hours(2);
    assert_eq!(next(now(), every, 1, Some(far)), far);
    let soon = now() + TimeDelta::minutes(1);
    assert_eq!(
        next(now(), every, 1, Some(soon)),
        now() + TimeDelta::minutes(30)
    );
}

fn headers(pairs: &[(&'static str, &str)]) -> Limits {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.insert(*name, HeaderValue::from_str(value).unwrap());
    }
    Limits::of(&map)
}

#[test]
fn src_021_a_rate_limit_ends_at_retry_after_then_at_the_reset_then_in_a_minute() {
    let reset = (now() + TimeDelta::minutes(20)).timestamp().to_string();
    let cases = [
        (
            403,
            headers(&[("retry-after", "120")]),
            Verdict::RateLimited(now() + TimeDelta::minutes(2)),
        ),
        (
            429,
            headers(&[
                ("x-ratelimit-remaining", "0"),
                ("x-ratelimit-reset", &reset),
                ("retry-after", "5"),
            ]),
            Verdict::RateLimited(now() + TimeDelta::seconds(5)),
        ),
        (
            403,
            headers(&[
                ("x-ratelimit-remaining", "0"),
                ("x-ratelimit-reset", &reset),
            ]),
            Verdict::RateLimited(now() + TimeDelta::minutes(20)),
        ),
        (
            429,
            headers(&[("x-ratelimit-remaining", "0")]),
            Verdict::RateLimited(now() + TimeDelta::minutes(1)),
        ),
        (
            403,
            headers(&[("retry-after", "999999999")]),
            Verdict::RateLimited(now() + TimeDelta::hours(24)),
        ),
        (
            403,
            headers(&[("x-ratelimit-remaining", "12")]),
            Verdict::Usable,
        ),
        (403, headers(&[]), Verdict::Usable),
        (401, headers(&[("retry-after", "5")]), Verdict::Refused),
        (500, headers(&[("retry-after", "5")]), Verdict::Usable),
        (
            200,
            headers(&[("x-ratelimit-remaining", "0")]),
            Verdict::Usable,
        ),
    ];
    for (status, limits, want) in cases {
        assert_eq!(judge(status, limits, now()), want, "{status} {limits:?}");
    }
}

#[test]
fn ext_002_only_accounts_whose_source_has_a_consumer_on_run_and_at_most_the_room() {
    let due = |source: &str| Due {
        id: SourceAccountId::from(Uuid::now_v7()),
        source: source.to_owned(),
    };
    let accounts = [due("github"), due("other"), due("github"), due("github")];
    let on = |source: &str| source == "github";
    let picked = runnable(&accounts, on, 2);
    assert_eq!(picked, [accounts[0].id, accounts[2].id]);
    assert_eq!(runnable(&accounts, |_| false, 4).len(), 0);
}

fn account() -> Account {
    Account {
        id: SourceAccountId::from(Uuid::now_v7()),
        source: "github".to_owned(),
        label: "Mine".to_owned(),
        instance_key: "1/2".to_owned(),
        credential_id: None,
        last_started_at: None,
        last_sync_at: None,
        last_error_kind: None,
        last_error: None,
        failure_count: 0,
        next_sync_at: now(),
        paused_at: None,
        run_generation: 0,
        run_until: None,
        created_at: now(),
    }
}

#[test]
fn itg_003_sync_now_is_refused_while_paused_or_with_no_consumer_and_never_starts_twice() {
    let paused = Account {
        paused_at: Some(now()),
        ..account()
    };
    assert!(matches!(sync_now(&paused, true, now()), Err(Error::Paused)));
    assert!(matches!(
        sync_now(&account(), false, now()),
        Err(Error::NoConsumer)
    ));
    assert!(matches!(sync_now(&account(), true, now()), Ok(true)));
    let running = Account {
        run_until: Some(now() + TimeDelta::minutes(1)),
        ..account()
    };
    assert!(matches!(sync_now(&running, true, now()), Ok(false)));
    let ran_out = Account {
        run_until: Some(now() - TimeDelta::minutes(1)),
        ..account()
    };
    assert!(
        matches!(sync_now(&ran_out, true, now()), Ok(true)),
        "a claim that ran out"
    );
}
