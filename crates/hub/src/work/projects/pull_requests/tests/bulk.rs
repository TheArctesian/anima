//! A bulk merge's outcomes for each refusal, its checked input and its deadline (PRJ-033).

use chrono::TimeDelta;

use super::{
    super::{
        bulk::{self, MergeOutcome},
        input::{self, PullRequestTargetInput},
        merge::Refusal,
    },
    rule::{HEAD, OTHER, at},
};
use crate::api_errors::{ErrorCode, coded};

#[test]
fn prj_033_each_refusal_has_its_outcome_and_code() {
    for refusal in [
        Refusal::ForkInBulk,
        Refusal::ForkUnconfirmed,
        Refusal::ForkByAgent,
    ] {
        assert_eq!(refusal.outcome(), MergeOutcome::ForkRefused);
        assert_eq!(refusal.code().as_str(), "FORBIDDEN");
    }
    assert_eq!(Refusal::HeadMoved.outcome(), MergeOutcome::HeadMoved);
    assert_eq!(Refusal::NotReady.outcome(), MergeOutcome::NotReady);
    assert_eq!(Refusal::NoMethods.outcome(), MergeOutcome::MethodRefused);
    assert_eq!(Refusal::Blocked.outcome(), MergeOutcome::NotMergeable);
    assert_eq!(Refusal::Behind.code().as_str(), "CONFLICT");
    assert!(!bulk::is_internal(&async_graphql::Error::new("x")));
    assert!(!bulk::is_internal(&coded("x", ErrorCode::Conflict)));
    assert!(bulk::is_internal(&coded("x", ErrorCode::Internal)));
}

#[test]
fn prj_033_the_bulk_deadline_is_90_seconds_from_the_start() {
    let started = at();
    assert!(!bulk::past(started, started));
    assert!(!bulk::past(started, started + TimeDelta::seconds(89)));
    assert!(bulk::past(started, started + TimeDelta::seconds(90)));
}

fn target(repository: &str, number: i32, sha: &str) -> PullRequestTargetInput {
    PullRequestTargetInput {
        repository: repository.into(),
        number,
        expected_head_sha: sha.to_owned(),
    }
}

#[test]
fn api_006_a_bulk_merge_takes_1_to_20_pull_requests_each_once() {
    let repository = "0190a000-0000-7000-8000-0000000000aa";
    let other = "0190a000-0000-7000-8000-0000000000bb";
    let fine = [
        target(repository, 1, HEAD),
        target(repository, 2, HEAD),
        target(other, 1, HEAD),
    ];
    let numbers: Vec<u64> = input::many(&fine)
        .unwrap()
        .iter()
        .map(|e| e.seen.number.get())
        .collect();
    assert_eq!(numbers, [1, 2, 1], "in the order given");
    let twenty: Vec<_> = (1..=20).map(|n| target(repository, n, HEAD)).collect();
    assert!(input::many(&twenty).is_ok());
    let refused = [
        Vec::new(),
        (1..=21).map(|n| target(repository, n, HEAD)).collect(),
        vec![target(repository, 1, HEAD), target(repository, 1, OTHER)],
        vec![target(repository, 0, HEAD)],
        vec![target(repository, 1, "abc")],
        vec![target("not an id", 1, HEAD)],
    ];
    for list in refused {
        let fields = format!("{:?}", input::many(&list).unwrap_err().extensions);
        assert!(fields.contains("pullRequests"), "{fields}");
    }
}
