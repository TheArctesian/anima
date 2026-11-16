//! K-E1 Section 13.1, decision 5: live reads stop at the sync's reserve, from the headers seen.

use chrono::TimeDelta;

use super::{account, now};
use crate::engine::sources::{
    acquisition::verdict::Limits,
    live::budget::{Budget, RESERVE, limited_until},
};

fn limits(remaining: u64, reset_in: i64) -> Limits {
    Limits {
        remaining: Some(remaining),
        reset: Some((now() + TimeDelta::seconds(reset_in)).timestamp()),
        retry_after: None,
    }
}

#[test]
fn prj_032_reads_go_above_the_reserve_and_stop_at_it_until_the_reset() {
    let mut budget = Budget::default();
    let id = account();
    assert_eq!(budget.read(id, now()), Ok(()), "nothing seen yet");

    budget.note(id, limits(RESERVE + 1, 600), now());
    assert_eq!(budget.read(id, now()), Ok(()));

    budget.note(id, limits(RESERVE, 600), now());
    let reset = now() + TimeDelta::seconds(600);
    assert_eq!(budget.read(id, now()), Err(reset));
    assert_eq!(budget.read(id, reset), Ok(()), "the count starts again");
}

#[test]
fn prj_032_the_last_answer_wins_and_no_total_is_assumed() {
    let mut budget = Budget::default();
    let id = account();
    budget.note(id, limits(400, 600), now());
    // A larger installation, or another region: the count may rise.
    budget.note(id, limits(12_000, 600), now());
    assert_eq!(budget.read(id, now()), Ok(()));

    budget.note(id, Limits::default(), now());
    assert_eq!(budget.read(id, now()), Ok(()), "no count, no change");
}

#[test]
fn prj_032_a_rate_limit_stops_reads_until_it_ends_and_a_missing_reset_waits_a_minute() {
    let mut budget = Budget::default();
    let (limited, unnamed) = (account(), account());
    let until = now() + TimeDelta::minutes(7);
    budget.limited(limited, until);
    budget.note(
        unnamed,
        Limits {
            remaining: Some(0),
            ..Limits::default()
        },
        now(),
    );

    assert_eq!(budget.read(limited, now()), Err(until));
    assert_eq!(
        budget.read(unnamed, now()),
        Err(now() + TimeDelta::minutes(1))
    );
    assert_eq!(budget.read(account(), now()), Ok(()), "other accounts go");
}

#[test]
fn prj_032_a_429_or_a_403_with_none_left_or_retry_after_is_a_limit_and_another_403_is_not() {
    let reset = now() + TimeDelta::minutes(9);
    let with = |remaining, retry_after| Limits {
        remaining,
        reset: Some(reset.timestamp()),
        retry_after,
    };
    assert_eq!(
        limited_until(429, Limits::default(), now()),
        Some(now() + TimeDelta::minutes(1))
    );
    assert_eq!(limited_until(429, with(Some(10), None), now()), Some(reset));
    assert_eq!(limited_until(403, with(Some(0), None), now()), Some(reset));
    assert_eq!(
        limited_until(403, with(Some(10), Some(30)), now()),
        Some(now() + TimeDelta::seconds(30))
    );
    assert_eq!(limited_until(403, with(Some(10), None), now()), None);
    assert_eq!(limited_until(403, Limits::default(), now()), None);
    assert_eq!(limited_until(200, with(Some(0), None), now()), None);
}
