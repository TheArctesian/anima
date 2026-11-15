//! K-E1 Section 13.1, decisions 5 and 7: live reads stop at the sync's reserve of the count in
//! the headers, as does a change that keeps the reserve, while the operator's change still
//! goes; a 429, a 403 with no request left and a 403 with `retry-after` are limits while any
//! other 403 is the provider's to read.

use anima_hub::engine::sources::{Keep, LiveFailure, Reserve, ask, change};
use chrono::{DateTime, TimeDelta, Utc};
use source_exchange::{Receipt, Settled};
use sqlx::PgPool;

use super::{COMMENT_PATH, PULLS, comment, get, live_kit, reply};
use crate::hub::start;

const KEEP: Keep = Keep::for_seconds(30);
const ISSUES: &str = "/repos/octo/widgets/issues";

fn reset() -> DateTime<Utc> {
    start() + TimeDelta::minutes(20)
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_reads_stop_at_500_left_until_the_reset_and_a_change_still_goes(pool: PgPool) {
    let (kit, id) = live_kit(&pool).await;
    let at = reset().timestamp().to_string();
    let left = |n: &str| {
        reply(
            200,
            &[
                ("x-ratelimit-limit", "12500"),
                ("x-ratelimit-remaining", n),
                ("x-ratelimit-reset", &at),
            ],
            "[]",
        )
    };
    kit.stub.reply(PULLS, left("501"));
    ask(&kit.at(start()), id, &get(PULLS), KEEP).await.unwrap();
    kit.stub.reply(ISSUES, left("500"));
    ask(&kit.at(start()), id, &get(ISSUES), KEEP.fresh(true))
        .await
        .unwrap();

    let refused = ask(
        &kit.at(start()),
        id,
        &get("/repos/octo/widgets/branches"),
        KEEP,
    )
    .await;
    assert_eq!(
        refused.unwrap_err(),
        LiveFailure::Limited { until: reset() }
    );
    assert_eq!(
        kit.stub.gets(),
        [PULLS, ISSUES],
        "the refused read was not sent"
    );

    kit.stub.reply(COMMENT_PATH, reply(201, &[], "{}"));
    let kept = change(&kit.at(start()), id, &comment("hello"), Reserve::Kept).await;
    assert_eq!(kept.unwrap_err(), LiveFailure::Limited { until: reset() });
    assert!(
        !kit.stub.paths().iter().any(|path| path == COMMENT_PATH),
        "a change that keeps the reserve was not sent"
    );
    let made = change(&kit.at(start()), id, &comment("hello"), Reserve::Spendable)
        .await
        .unwrap();
    assert_eq!(made, Settled::Done(Receipt::Created { number: 201 }));

    let after = ask(
        &kit.at(reset()),
        id,
        &get("/repos/octo/widgets/branches"),
        KEEP,
    )
    .await;
    assert_eq!(
        after.unwrap().response().status,
        404,
        "the count started again"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn prj_032_a_429_or_a_403_without_requests_left_is_a_limit_and_another_403_is_not(
    pool: PgPool,
) {
    let (kit, id) = live_kit(&pool).await;
    // Each case starts an hour after the last, once the last limit is over.
    let hour = |n: i32| start() + TimeDelta::hours(n.into());
    let reset_of = |n: i32| (hour(n) + TimeDelta::minutes(20)).timestamp().to_string();
    let (b, c) = (reset_of(1), reset_of(2));
    let cases = [
        ("/a", 429, vec![], hour(0) + TimeDelta::minutes(1)),
        (
            "/b",
            429,
            vec![("x-ratelimit-reset", b.as_str())],
            hour(1) + TimeDelta::minutes(20),
        ),
        (
            "/c",
            403,
            vec![
                ("x-ratelimit-remaining", "0"),
                ("x-ratelimit-reset", c.as_str()),
            ],
            hour(2) + TimeDelta::minutes(20),
        ),
        (
            "/d",
            403,
            vec![("retry-after", "30")],
            hour(3) + TimeDelta::seconds(30),
        ),
    ];
    for (n, (path, status, headers, until)) in (0..).zip(cases) {
        kit.stub
            .reply(path, reply(status, &headers, "{\"message\":\"limited\"}"));
        let limited = ask(&kit.at(hour(n)), id, &get(path), KEEP).await;
        assert_eq!(
            limited.unwrap_err(),
            LiveFailure::Limited { until },
            "{path}"
        );
        let held_back = ask(&kit.at(hour(n)), id, &get("/e"), KEEP).await;
        assert_eq!(
            held_back.unwrap_err(),
            LiveFailure::Limited { until },
            "{path}"
        );
        assert_eq!(
            kit.stub.gets().last().map(String::as_str),
            Some(path),
            "/e was not sent"
        );
    }

    kit.stub.reply(
        "/f",
        reply(403, &[], "{\"message\":\"Resource not accessible\"}"),
    );
    let refused = ask(&kit.at(hour(4)), id, &get("/f"), KEEP).await;
    assert_eq!(refused.unwrap().response().status, 403);
}
