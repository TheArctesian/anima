//! What each reply status means for each change (design C.6, decisions items 2 and 7).

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};
use source_exchange::{Body, Change, ChangeReply, Limit, Receipt, Refusal, Settled};

use super::{every_change, sha};
use crate::changes::settle;
use crate::tests::fixture;

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 14, 12, 0, 0).unwrap()
}

fn seconds_after(seconds: i64) -> DateTime<Utc> {
    at() + TimeDelta::seconds(seconds)
}

fn reply(status: u16, body: Body<'_>, limit: Limit) -> ChangeReply<'_> {
    ChangeReply {
        status,
        body,
        limit,
        at: at(),
    }
}

fn settled(change: &Change, status: u16, body: &[u8]) -> Settled {
    settle(change, &reply(status, Body::Read(body), Limit::default()))
}

const fn refused(refusal: Refusal) -> Settled {
    Settled::Refused(refusal)
}

const MADE: Settled = Settled::Done(Receipt::Made);

/// Rows of status, body and outcome for one change, with no rate limit headers.
fn assert_table(change: &Change, rows: &[(u16, &[u8], Settled)]) {
    for (status, body, expected) in rows {
        assert_eq!(
            &settled(change, *status, body),
            expected,
            "{} {status}",
            change.function()
        );
    }
}

#[test]
fn prj_031_create_issue_is_done_with_the_new_number_on_201() {
    let issue = fixture("change_issue.json");
    assert_table(
        &every_change()[0],
        &[
            (
                201,
                &issue,
                Settled::Done(Receipt::Created { number: 1347 }),
            ),
            (201, b"{not json", MADE),
            (201, br#"{"number":0}"#, MADE),
            (200, &issue, Settled::Unknown),
            (405, b"", refused(Refusal::Unexplained)),
            (409, b"", refused(Refusal::Unexplained)),
            (422, b"", refused(Refusal::Invalid)),
        ],
    );
}

#[test]
fn prj_031_a_comment_is_done_with_its_id_on_201() {
    let comment = fixture("change_comment.json");
    assert_table(
        &every_change()[1],
        &[
            (
                201,
                &comment,
                Settled::Done(Receipt::Created {
                    number: 1_253_456_789,
                }),
            ),
            (201, b"[]", MADE),
            (202, &comment, Settled::Unknown),
            (405, b"", refused(Refusal::Unexplained)),
            (409, b"", refused(Refusal::Unexplained)),
            (422, b"", refused(Refusal::Invalid)),
        ],
    );
}

#[test]
fn prj_031_set_issue_open_is_done_on_200() {
    let issue = fixture("change_issue.json");
    assert_table(
        &every_change()[2],
        &[
            (200, &issue, MADE),
            (200, b"<html>", MADE),
            (201, &issue, Settled::Unknown),
            (405, b"", refused(Refusal::Unexplained)),
            (409, b"", refused(Refusal::Unexplained)),
            (422, b"", refused(Refusal::Invalid)),
        ],
    );
}

#[test]
fn prj_033_a_merge_is_done_with_the_merged_commit_and_refused_on_405_409_422() {
    let merged = fixture("change_merged.json");
    assert_table(
        &every_change()[3],
        &[
            (200, &merged, Settled::Done(Receipt::Merged { sha: sha() })),
            (200, br#"{"merged":true,"sha":"not a commit"}"#, MADE),
            (200, b"garbage", MADE),
            (200, br#"{"merged":false,"message":"no"}"#, Settled::Unknown),
            (201, &merged, Settled::Unknown),
            (405, b"", refused(Refusal::NotMergeable)),
            (409, b"", refused(Refusal::HeadMoved)),
            (422, b"", refused(Refusal::NotMergeable)),
        ],
    );
}

#[test]
fn prj_034_running_again_is_done_on_201() {
    for change in &every_change()[4..=5] {
        assert_table(
            change,
            &[
                (201, b"", MADE),
                (201, b"{\"unexpected\":", MADE),
                (200, b"", Settled::Unknown),
                (405, b"", refused(Refusal::Unexplained)),
                (409, b"", refused(Refusal::Unexplained)),
                (422, b"", refused(Refusal::Invalid)),
            ],
        );
    }
}

#[test]
fn prj_034_a_cancel_is_done_on_202_and_a_conflict_on_409() {
    assert_table(
        &every_change()[6],
        &[
            (202, b"{}", MADE),
            (202, b"<", MADE),
            (200, b"{}", Settled::Unknown),
            (405, b"", refused(Refusal::Unexplained)),
            (409, b"", refused(Refusal::Conflict)),
            (422, b"", refused(Refusal::Invalid)),
        ],
    );
}

#[test]
fn prj_035_a_dispatch_is_done_on_204_and_on_200_with_the_run() {
    let run = fixture("change_dispatch_run.json");
    assert_table(
        &every_change()[7],
        &[
            (204, b"", MADE),
            (200, &run, MADE),
            (200, b"garbage", MADE),
            (201, b"", Settled::Unknown),
            (405, b"", refused(Refusal::Unexplained)),
            (409, b"", refused(Refusal::Unexplained)),
            (422, b"", refused(Refusal::Invalid)),
        ],
    );
}

#[test]
fn pro_010_a_commit_status_is_done_on_201() {
    let status = fixture("change_status.json");
    assert_table(
        &every_change()[8],
        &[
            (201, &status, MADE),
            (201, b"[", MADE),
            (200, &status, Settled::Unknown),
            (405, b"", refused(Refusal::Unexplained)),
            (409, b"", refused(Refusal::Unexplained)),
            (422, b"", refused(Refusal::Invalid)),
        ],
    );
}

#[test]
fn od_e14_a_success_body_too_large_to_read_is_still_done() {
    let success = [201, 201, 200, 200, 201, 201, 202, 204, 201];
    for (change, status) in every_change().iter().zip(success) {
        let settled = settle(change, &reply(status, Body::TooLarge, Limit::default()));
        assert_eq!(settled, MADE, "{}", change.function());
    }
}

fn spent(reset_after: i64) -> Limit {
    Limit {
        remaining: Some(0),
        reset_at: Some(seconds_after(reset_after)),
        retry_after_seconds: None,
    }
}

fn retry_after(seconds: u64) -> Limit {
    Limit {
        retry_after_seconds: Some(seconds),
        ..Limit::default()
    }
}

#[test]
fn od_e14_the_statuses_every_change_shares() {
    let not_spent = Limit {
        remaining: Some(4999),
        reset_at: Some(seconds_after(900)),
        retry_after_seconds: None,
    };
    let limited = |seconds| Settled::Limited {
        until: seconds_after(seconds),
    };
    let rows = [
        (401, Limit::default(), refused(Refusal::KeyRefused)),
        (403, Limit::default(), refused(Refusal::NotPermitted)),
        (403, not_spent, refused(Refusal::NotPermitted)),
        (403, spent(1800), limited(1800)),
        (403, retry_after(30), limited(30)),
        (404, Limit::default(), refused(Refusal::NotFound)),
        (410, Limit::default(), refused(Refusal::Gone)),
        (429, Limit::default(), limited(60)),
        (429, not_spent, limited(60)),
        (429, spent(1800), limited(1800)),
        (429, retry_after(90), limited(90)),
        (400, Limit::default(), refused(Refusal::Unexplained)),
        (301, Limit::default(), refused(Refusal::Unexplained)),
        (500, Limit::default(), Settled::Unknown),
        (502, Limit::default(), Settled::Unknown),
        (503, Limit::default(), Settled::Unknown),
    ];
    for change in &every_change() {
        for (status, limit, expected) in &rows {
            let got = settle(change, &reply(*status, Body::Read(b"{}"), *limit));
            assert_eq!(&got, expected, "{} {status} {limit:?}", change.function());
        }
    }
}

#[test]
fn od_e14_a_limit_waits_for_the_later_time_and_never_for_a_past_one() {
    let change = &every_change()[0];
    let limited = |limit: Limit| settle(change, &reply(403, Body::Read(b""), limit));
    let both = Limit {
        retry_after_seconds: Some(10),
        ..spent(300)
    };
    let until = |seconds| Settled::Limited {
        until: seconds_after(seconds),
    };
    assert_eq!(limited(both), until(300));
    assert_eq!(limited(spent(-5)), until(60), "a reset already past");
    assert_eq!(limited(retry_after(0)), until(60));
    assert_eq!(
        limited(retry_after(u64::MAX)),
        Settled::Limited {
            until: DateTime::<Utc>::MAX_UTC
        },
        "a wait too long to count does not panic"
    );
}

#[test]
fn log_007_no_text_of_a_reply_reaches_the_outcome() {
    let secret = "ghs_secretsecret and a message from GitHub";
    let body = format!(r#"{{"message":"{secret}","sha":"{secret}","number":"{secret}"}}"#);
    for change in &every_change() {
        for status in [200, 201, 202, 204, 403, 404, 405, 409, 422, 500] {
            let settled = settled(change, status, body.as_bytes());
            assert!(!format!("{settled:?}").contains("secret"), "{settled:?}");
        }
    }
}
